use forester::photon_indexer::PhotonIndexer;
use forester::queue_helpers::fetch_queue_item_data;
use forester::rpc_pool::SolanaRpcPool;
use forester::run_pipeline;
use forester::utils::LightValidatorConfig;
use light_registry::utils::{get_epoch_pda_address, get_forester_epoch_pda_from_authority};
use light_registry::{EpochPda, ForesterEpochPda};
use light_test_utils::e2e_test_env::E2ETestEnv;
use light_test_utils::indexer::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts, TestIndexer};
use light_test_utils::registry::register_test_forester;
use light_test_utils::rpc::errors::RpcError;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::solana_rpc::SolanaRpcUrl;
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::EnvAccounts;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::{sleep, timeout};

mod test_utils;
use test_utils::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn test_epoch_monitor_with_test_indexer_and_1_forester() {
    init(Some(LightValidatorConfig {
        enable_indexer: false,
        enable_prover: true,
        enable_forester: false,
        wait_time: 10,
        ..LightValidatorConfig::default()
    }))
    .await;

    let forester_keypair = Keypair::new();

    let mut env_accounts = EnvAccounts::get_local_test_validator_accounts();
    env_accounts.forester = forester_keypair.insecure_clone();

    let mut config = forester_config();
    config.payer_keypair = forester_keypair.insecure_clone();

    let config = Arc::new(config);
    let pool = SolanaRpcPool::<SolanaRpcConnection>::new(
        config.external_services.rpc_url.to_string(),
        CommitmentConfig::confirmed(),
        config.rpc_pool_size as u32,
    )
    .await
    .unwrap();

    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
    rpc.payer = forester_keypair.insecure_clone();

    rpc.airdrop_lamports(&forester_keypair.pubkey(), LAMPORTS_PER_SOL * 100_000)
        .await
        .unwrap();

    rpc.airdrop_lamports(
        &env_accounts.governance_authority.pubkey(),
        LAMPORTS_PER_SOL * 100_000,
    )
    .await
    .unwrap();

    register_test_forester(
        &mut rpc,
        &env_accounts.governance_authority,
        &forester_keypair.pubkey(),
        light_registry::ForesterConfig::default(),
    )
    .await
    .unwrap();

    let indexer: TestIndexer<SolanaRpcConnection> =
        TestIndexer::init_from_env(&config.payer_keypair, &env_accounts, false, false).await;

    let mut env = E2ETestEnv::<SolanaRpcConnection, TestIndexer<SolanaRpcConnection>>::new(
        rpc,
        indexer,
        &env_accounts,
        keypair_action_config(),
        general_action_config(),
        0,
        Some(0),
    )
    .await;

    let user_index = 0;
    let balance = env
        .rpc
        .get_balance(&env.users[user_index].keypair.pubkey())
        .await
        .unwrap();
    env.compress_sol(user_index, balance).await;
    let state_trees: Vec<StateMerkleTreeAccounts> = env
        .indexer
        .state_merkle_trees
        .iter()
        .map(|x| x.accounts)
        .collect();
    let address_trees: Vec<AddressMerkleTreeAccounts> = env
        .indexer
        .address_merkle_trees
        .iter()
        .map(|x| x.accounts)
        .collect();

    let iterations = 1;
    let mut total_expected_work = 0;
    // Create work and assert that the queues are not empty
    {
        for _ in 0..iterations {
            env.transfer_sol(user_index).await;
            env.create_address(None).await;
        }

        // Asserting non empty because transfer sol is not deterministic.
        assert_queue_len(
            &pool,
            &state_trees,
            &address_trees,
            &mut total_expected_work,
            iterations,
            true,
        )
        .await;
    }

    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    let (work_report_sender, mut work_report_receiver) = mpsc::channel(100);

    // Run the forester as pipeline
    let service_handle = tokio::spawn(run_pipeline(
        config.clone(),
        Arc::new(Mutex::new(env.indexer)),
        shutdown_receiver,
        work_report_sender,
    ));

    if let Some(_) = work_report_receiver.recv().await {
        println!("work_reported");
    };
    let mut rpc = pool.get_connection().await.unwrap();
    let epoch_pda_address = get_epoch_pda_address(0);
    let epoch_pda = (*rpc)
        .get_anchor_account::<EpochPda>(&epoch_pda_address)
        .await
        .unwrap()
        .unwrap();
    let total_processed = epoch_pda.total_work;

    let forester_epoch_pda_address =
        get_forester_epoch_pda_from_authority(&config.payer_keypair.pubkey(), 0).0;
    let forester_epoch_pda = (*rpc)
        .get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_address)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(forester_epoch_pda.work_counter, total_processed);

    // assert that all (2) queues have been emptied
    {
        assert_queue_len(
            &pool,
            &state_trees.clone(),
            &address_trees.clone(),
            &mut 0,
            0,
            false,
        )
        .await;

        assert_eq!(
            total_processed, total_expected_work,
            "Not all items were processed."
        );
    }
    shutdown_sender
        .send(())
        .expect("Failed to send shutdown signal");
    service_handle.await.unwrap().unwrap();
}

pub async fn assert_queue_len(
    pool: &SolanaRpcPool<SolanaRpcConnection>,
    state_trees: &Vec<StateMerkleTreeAccounts>,
    address_trees: &Vec<AddressMerkleTreeAccounts>,
    total_expected_work: &mut u64,
    expected_len: usize,
    not_empty: bool,
) {
    for tree in state_trees.iter() {
        let mut rpc = pool.get_connection().await.unwrap();
        let queue_length = fetch_queue_item_data(&mut *rpc, &tree.nullifier_queue)
            .await
            .unwrap()
            .len();
        if not_empty {
            assert_ne!(queue_length, 0);
        } else {
            assert_eq!(queue_length, expected_len);
        }
        *total_expected_work += queue_length as u64;
    }

    for tree in address_trees.iter() {
        let mut rpc = pool.get_connection().await.unwrap();
        let queue_length = fetch_queue_item_data(&mut *rpc, &tree.queue)
            .await
            .unwrap()
            .len();
        if not_empty {
            assert_ne!(queue_length, 0);
        } else {
            assert_eq!(queue_length, expected_len);
        }
        *total_expected_work += queue_length as u64;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_with_photon() {
    init(Some(LightValidatorConfig {
        enable_indexer: true,
        enable_prover: true,
        enable_forester: false,
        wait_time: 10,
        ..LightValidatorConfig::default()
    }))
    .await;
    let photon_indexer = create_local_photon_indexer();
    photon_indexer
        .get_rpc_compressed_accounts_by_owner(&photon_indexer.payer.pubkey())
        .await
        .unwrap();
}

pub fn create_local_photon_indexer() -> PhotonIndexer<SolanaRpcConnection> {
    let rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
    PhotonIndexer::new(String::from("127.0. 0.1:3001"), "", rpc)
}

pub fn create_test_forester() {
    let forester_keypair1 = Keypair::new();
    let mut env_accounts = EnvAccounts::get_local_test_validator_accounts();

    let mut config1 = forester_config();
    env_accounts.forester = forester_keypair1.insecure_clone();
    let config1 = Arc::new(config1);
}

// TODO: add test which asserts epoch registration over many epochs (we need a different protocol config for that)
// TODO: add test with photon indexer for an infitine local test which performs work over many epochs
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_epoch_monitor_with_2_foresters() {
    init(Some(LightValidatorConfig {
        enable_indexer: false,
        enable_prover: true,
        enable_forester: false,
        wait_time: 10,
        ..LightValidatorConfig::default()
    }))
    .await;
    let forester_keypair1 = Keypair::new();
    let forester_keypair2 = Keypair::new();

    let mut env_accounts = EnvAccounts::get_local_test_validator_accounts();
    env_accounts.forester = forester_keypair1.insecure_clone();

    let mut config1 = forester_config();
    config1.payer_keypair = forester_keypair1.insecure_clone();
    let config1 = Arc::new(config1);

    let mut config2 = forester_config();
    config2.payer_keypair = forester_keypair2.insecure_clone();
    let config2 = Arc::new(config2);

    let pool = SolanaRpcPool::<SolanaRpcConnection>::new(
        config1.external_services.rpc_url.to_string(),
        CommitmentConfig::confirmed(),
        config1.rpc_pool_size as u32,
    )
    .await
    .unwrap();

    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
    rpc.payer = forester_keypair1.insecure_clone();

    // Airdrop to both foresters and governance authority
    for keypair in [
        &forester_keypair1,
        &forester_keypair2,
        &env_accounts.governance_authority,
    ] {
        rpc.airdrop_lamports(&keypair.pubkey(), LAMPORTS_PER_SOL * 100_000)
            .await
            .unwrap();
    }

    // Register both foresters
    for forester_keypair in [&forester_keypair1, &forester_keypair2] {
        register_test_forester(
            &mut rpc,
            &env_accounts.governance_authority,
            &forester_keypair.pubkey(),
            light_registry::ForesterConfig::default(),
        )
        .await
        .unwrap();
    }

    let indexer: TestIndexer<SolanaRpcConnection> =
        TestIndexer::init_from_env(&config1.payer_keypair, &env_accounts, false, false).await;

    let mut env = E2ETestEnv::<SolanaRpcConnection, TestIndexer<SolanaRpcConnection>>::new(
        rpc,
        indexer,
        &env_accounts,
        keypair_action_config(),
        general_action_config(),
        0,
        Some(0),
    )
    .await;

    let user_index = 0;
    let balance = env
        .rpc
        .get_balance(&env.users[user_index].keypair.pubkey())
        .await
        .unwrap();
    env.compress_sol(user_index, balance).await;

    let state_trees: Vec<StateMerkleTreeAccounts> = env
        .indexer
        .state_merkle_trees
        .iter()
        .map(|x| x.accounts)
        .collect();
    let address_trees: Vec<AddressMerkleTreeAccounts> = env
        .indexer
        .address_merkle_trees
        .iter()
        .map(|x| x.accounts)
        .collect();
    let mut total_expected_work = 0;
    {
        let iterations = 5;
        for i in 0..iterations {
            println!("Round {} of {}", i, iterations);
            env.transfer_sol(user_index).await;
            sleep(Duration::from_millis(100)).await;
            env.create_address(None).await;
        }
        assert_queue_len(
            &pool,
            &state_trees,
            &address_trees,
            &mut total_expected_work,
            0,
            true,
        )
        .await;
    }

    let (shutdown_sender1, shutdown_receiver1) = oneshot::channel();
    let (shutdown_sender2, shutdown_receiver2) = oneshot::channel();
    let (work_report_sender1, mut work_report_receiver1) = mpsc::channel(100);
    let (work_report_sender2, mut work_report_receiver2) = mpsc::channel(100);

    let indexer = Arc::new(Mutex::new(env.indexer));

    let service_handle1 = tokio::spawn(run_pipeline(
        config1.clone(),
        indexer.clone(),
        shutdown_receiver1,
        work_report_sender1,
    ));
    let service_handle2 = tokio::spawn(run_pipeline(
        config2.clone(),
        indexer,
        shutdown_receiver2,
        work_report_sender2,
    ));

    let mut forester1_reported_work_for_epoch1 = false;
    let mut forester2_reported_work_for_epoch1 = false;

    // Wait for both foresters to report work for epoch 1
    const TIMEOUT_DURATION: Duration = Duration::from_secs(360);
    let mut total_processed = 0;
    let result = timeout(TIMEOUT_DURATION, async {
        loop {
            tokio::select! {
            Some(report) = work_report_receiver1.recv(), if !forester1_reported_work_for_epoch1 => {
                    total_processed += report.processed_items;
                if report.epoch == 1 {
                    forester1_reported_work_for_epoch1 = true;
                }
            }
            Some(report) = work_report_receiver2.recv(), if !forester2_reported_work_for_epoch1 => {
                    total_processed += report.processed_items;
                if report.epoch == 1 {
                    forester2_reported_work_for_epoch1 = true;
                }
            }
            else => break,
        }
            if forester1_reported_work_for_epoch1 && forester2_reported_work_for_epoch1 {
                break;
            }
        }
        total_processed
    }).await;

    match result {
        Ok(total_processed) => {
            assert!(total_processed > 0, "No items were processed");
            println!(
                "Both foresters reported work for epoch 1. Total processed: {}",
                total_processed
            );
        }
        Err(_) => {
            panic!("Test timed out after {:?}", TIMEOUT_DURATION);
        }
    }

    // assert queues have been emptied
    assert_queue_len(&pool, &state_trees, &address_trees, &mut 0, 0, false).await;
    let mut rpc = pool.get_connection().await.unwrap();
    let forester_pubkeys = [
        config1.payer_keypair.pubkey(),
        config2.payer_keypair.pubkey(),
    ];
    // assert epoch 0
    {
        let total_processed_work = assert_foresters_registered(&forester_pubkeys[..], &mut rpc, 0)
            .await
            .unwrap();
        assert_eq!(
            total_processed_work, total_expected_work,
            "Not all items were processed."
        );
    }
    // assert that foresters registered for epoch 1 and 2 (no new work is emitted after epoch 0)
    for epoch in 1..=2 {
        let total_processed_work =
            assert_foresters_registered(&forester_pubkeys[..], &mut rpc, epoch)
                .await
                .unwrap();
        assert_eq!(
            total_processed_work, 0,
            "Not all items were processed in prior epoch."
        );
    }
    shutdown_sender1
        .send(())
        .expect("Failed to send shutdown signal to forester 1");
    shutdown_sender2
        .send(())
        .expect("Failed to send shutdown signal to forester 2");
    service_handle1.await.unwrap().unwrap();
    service_handle2.await.unwrap().unwrap();
}

async fn assert_foresters_registered(
    foresters: &[Pubkey],
    rpc: &mut SolanaRpcConnection,
    epoch: u64,
) -> Result<u64, RpcError> {
    let mut performed_work = 0;
    for (i, forester) in foresters.iter().enumerate() {
        let forester_epoch_pda = get_forester_epoch_pda_from_authority(forester, epoch).0;
        let forester_epoch_pda = rpc
            .get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda)
            .await?;
        println!("forester_epoch_pda {}: {:?}", i, forester_epoch_pda);

        if let Some(forester_epoch_pda) = forester_epoch_pda {
            // If one forester is first for both queues there will be no work left
            // - this assert is flaky
            // assert!(
            //     forester_epoch_pda.work_counter > 0,
            //     "forester {} did not perform any work",
            //     i
            // );
            performed_work += forester_epoch_pda.work_counter;
        } else {
            return Err(RpcError::CustomError(format!(
                "Forester {} not registered",
                i,
            )));
        }
    }
    Ok(performed_work)
}
