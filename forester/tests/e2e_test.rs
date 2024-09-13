use account_compression::utils::constants::{ADDRESS_QUEUE_VALUES, STATE_NULLIFIER_QUEUE_VALUES};
use account_compression::AddressMerkleTreeAccount;
use forester::queue_helpers::fetch_queue_item_data;
use forester::run_pipeline;
use forester::utils::{get_protocol_config, LightValidatorConfig};
use forester_utils::indexer::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts};
use forester_utils::registry::register_test_forester;
use light_client::rpc::solana_rpc::SolanaRpcUrl;
use light_client::rpc::{RpcConnection, RpcError, SolanaRpcConnection};
use light_client::rpc_pool::SolanaRpcPool;
use light_registry::utils::{get_epoch_pda_address, get_forester_epoch_pda_from_authority};
use light_registry::{EpochPda, ForesterEpochPda};
use light_test_utils::e2e_test_env::E2ETestEnv;
use light_test_utils::indexer::TestIndexer;
use light_test_utils::test_env::EnvAccounts;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, timeout};

mod test_utils;
use test_utils::*;

//
// #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
// async fn test_epoch_explorer() {
//     let mut rpc = SolanaRpcConnection::new(
//         "https://devnet.helius-rpc.com/?api-key=27c42b89-12ac-41d0-8fa7-6341caa5737d",
//         None,
//     );
//
//     let trees = fetch_trees(&rpc).await;
//     for tree_data in trees {
//         let length = if tree_data.tree_type == TreeType::State {
//             STATE_NULLIFIER_QUEUE_VALUES
//         } else {
//             ADDRESS_QUEUE_VALUES
//         };
//
//         let mut account = rpc
//             .get_anchor_account
//             ::<AddressMerkleTreeAccount>
//             ()
//             .get_account(tree_data.merkle_tree)
//             .await
//             .unwrap();
//     }
//
//     let protocol_config = get_protocol_config(&mut rpc).await;
//     let solana_slot = rpc.get_slot().await.unwrap();
//
//     let epoch = protocol_config.get_current_epoch(solana_slot) - 1;
//
//     println!("Current solana slot: {}", solana_slot);
//     println!("Protocol config: {:?}", protocol_config);
//
//     println!("Current light epoch: {}", epoch);
//
//     let progress = protocol_config.get_current_active_epoch_progress(solana_slot);
//
//     let slots_left = protocol_config.active_phase_length - progress;
//     let slot_time = 0.43;
//     let time_left_seconds = slots_left as f64 * slot_time;
//     println!("Progress: {}", progress);
//     println!("Slots left: {}", slots_left);
//
//     let minutes = (time_left_seconds / 60.0).floor();
//     let hours = (minutes / 60.0).floor();
//     let remaining_minutes = minutes % 60.0;
//
//     if hours > 0.0 {
//         println!("Time left: {:.0} hours and {:.0} minutes", hours, remaining_minutes);
//     } else {
//         println!("Time left: {:.0} minutes", minutes);
//     }
//
//     let epoch_pda_address = get_epoch_pda_address(epoch);
//     let epoch_pda = rpc
//         .get_anchor_account::<EpochPda>(&epoch_pda_address)
//         .await
//         .unwrap().unwrap();
//
//     println!("Epoch PDA: {:?}", epoch_pda);
//
//
//     for keypair in mainnet_forester_keypairs {
//         let forester_keypair = Keypair::from_bytes(&keypair).unwrap();
//         let forester_epoch_pda_pubkey =  get_forester_epoch_pda_from_authority(&forester_keypair.pubkey(), epoch).0;
//         let existing_pda = rpc
//             .get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_pubkey)
//             .await
//             .unwrap();
//
//         match existing_pda {
//             Some(pda) => {
//                 println!("Forester {} PDA: {:?}", forester_keypair.pubkey(), pda);
//             }
//             None => {
//                 println!("Forester {} PDA not found.", forester_keypair.pubkey());
//             },
//         }
//     }
// }

#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
#[ignore]
async fn test_epoch_monitor_with_test_indexer_and_1_forester() {
    init(Some(LightValidatorConfig {
        enable_indexer: false,
        enable_prover: true,
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
        config.general_config.rpc_pool_size as u32,
    )
    .await
    .unwrap();

    let rpc = SolanaRpcConnection::new_with_retry(
        SolanaRpcUrl::Localnet,
        None,
        None,
        Some(forester_keypair.insecure_clone()),
    );

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
        &rpc,
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

    let state_trees: Vec<StateMerkleTreeAccounts> = {
        let state_merkle_trees = env.indexer.state.state_merkle_trees.read().await;
        state_merkle_trees.iter().map(|x| x.accounts).collect()
    };

    let address_trees: Vec<AddressMerkleTreeAccounts> = {
        let address_merkle_trees = env.indexer.state.address_merkle_trees.read().await;
        address_merkle_trees.iter().map(|x| x.accounts).collect()
    };

    let iterations = 1;
    let mut total_expected_work = 0;
    // Create work and assert that the queues are not empty
    {
        for _ in 0..iterations {
            env.transfer_sol(user_index).await;
            env.create_address(None, None).await;
        }

        // Asserting non-empty because transfer sol is not deterministic.
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
        Arc::new(env.indexer),
        shutdown_receiver,
        work_report_sender,
    ));

    if work_report_receiver.recv().await.is_some() {
        println!("work_reported");
    };
    let rpc = pool.get_connection().await.unwrap();
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
    state_trees: &[StateMerkleTreeAccounts],
    address_trees: &[AddressMerkleTreeAccounts],
    total_expected_work: &mut u64,
    expected_len: usize,
    not_empty: bool,
) {
    for tree in state_trees.iter() {
        let rpc = pool.get_connection().await.unwrap();
        let queue_length = fetch_queue_item_data(
            &*rpc,
            &tree.nullifier_queue,
            0,
            STATE_NULLIFIER_QUEUE_VALUES,
            STATE_NULLIFIER_QUEUE_VALUES,
        )
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
        let rpc = pool.get_connection().await.unwrap();
        let queue_length = fetch_queue_item_data(
            &*rpc,
            &tree.queue,
            0,
            ADDRESS_QUEUE_VALUES,
            ADDRESS_QUEUE_VALUES,
        )
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

// TODO: add test which asserts epoch registration over many epochs (we need a different protocol config for that)
// TODO: add test with photon indexer for an infinite local test which performs work over many epochs
#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
async fn test_epoch_monitor_with_2_foresters() {
    init(Some(LightValidatorConfig {
        enable_indexer: false,
        enable_prover: true,
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
        config1.general_config.rpc_pool_size as u32,
    )
    .await
    .unwrap();

    let rpc = SolanaRpcConnection::new_with_retry(
        SolanaRpcUrl::Localnet,
        None,
        None,
        Some(forester_keypair1.insecure_clone()),
    );

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
            &rpc,
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
    // Create state and address trees which can be rolled over
    env.create_address_tree(Some(0)).await;
    env.create_state_tree(Some(0)).await;
    let state_tree_with_rollover_threshold_0 = {
        let state_merkle_trees = env.indexer.state.state_merkle_trees.read().await;
        state_merkle_trees[1].accounts.merkle_tree
    };
    let address_tree_with_rollover_threshold_0 = {
        let address_merkle_trees = env.indexer.state.address_merkle_trees.read().await;
        address_merkle_trees[1].accounts.merkle_tree
    };
    let state_trees: Vec<StateMerkleTreeAccounts> = {
        let state_merkle_trees = env.indexer.state.state_merkle_trees.read().await;
        state_merkle_trees.iter().map(|x| x.accounts).collect()
    };
    let address_trees: Vec<AddressMerkleTreeAccounts> = {
        let address_merkle_trees = env.indexer.state.address_merkle_trees.read().await;
        address_merkle_trees.iter().map(|x| x.accounts).collect()
    };

    println!("Address trees: {:?}", address_trees);

    // Two rollovers plus other work
    let mut total_expected_work = 2;
    {
        let iterations = 5;
        for i in 0..iterations {
            println!("Round {} of {}", i, iterations);
            let user_keypair = env.users[0].keypair.insecure_clone();
            env.transfer_sol_deterministic(&user_keypair, &user_keypair.pubkey(), Some(1))
                .await
                .unwrap();
            env.transfer_sol_deterministic(&user_keypair, &user_keypair.pubkey().clone(), Some(0))
                .await
                .unwrap();
            sleep(Duration::from_millis(100)).await;
            env.create_address(None, Some(1)).await;
            env.create_address(None, Some(0)).await;
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

    let indexer = Arc::new(env.indexer);

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

    // Wait for both foresters to report work for epoch 1
    const TIMEOUT_DURATION: Duration = Duration::from_secs(360);
    const EXPECTED_EPOCHS: u64 = 2; // We expect to process 2 epochs (0 and 1)

    let result: Result<(), tokio::time::error::Elapsed> = timeout(TIMEOUT_DURATION, async {
        let mut processed_epochs = HashSet::new();
        let mut total_processed = 0;
        while processed_epochs.len() < EXPECTED_EPOCHS as usize {
            tokio::select! {
                Some(report) = work_report_receiver1.recv() => {
                    println!("Received work report from forester 1: {:?}", report);
                    total_processed += report.processed_items;
                    processed_epochs.insert(report.epoch);
                }
                Some(report) = work_report_receiver2.recv() => {
                    println!("Received work report from forester 2: {:?}", report);
                    total_processed += report.processed_items;
                    processed_epochs.insert(report.epoch);
                }
                else => break,
            }
        }

        println!("Processed {} items", total_processed);

        // Verify that we've processed the expected number of epochs
        assert_eq!(
            processed_epochs.len(),
            EXPECTED_EPOCHS as usize,
            "Processed {} epochs, expected {}",
            processed_epochs.len(),
            EXPECTED_EPOCHS
        );

        // Verify that we've processed epochs 0 and 1
        assert!(processed_epochs.contains(&0), "Epoch 0 was not processed");
        assert!(processed_epochs.contains(&1), "Epoch 1 was not processed");
    })
    .await;

    // Handle timeout
    if result.is_err() {
        panic!("Test timed out after {:?}", TIMEOUT_DURATION);
    }

    assert_trees_are_rollledover(
        &pool,
        &state_tree_with_rollover_threshold_0,
        &address_tree_with_rollover_threshold_0,
    )
    .await;
    // assert queues have been emptied
    assert_queue_len(&pool, &state_trees, &address_trees, &mut 0, 0, false).await;
    let rpc = pool.get_connection().await.unwrap();
    let forester_pubkeys = [
        config1.payer_keypair.pubkey(),
        config2.payer_keypair.pubkey(),
    ];

    // assert that foresters registered for epoch 1 and 2 (no new work is emitted after epoch 0)
    // Assert that foresters have registered all processed epochs and the next epoch (+1)
    for epoch in 0..=EXPECTED_EPOCHS {
        let total_processed_work = assert_foresters_registered(&forester_pubkeys[..], &rpc, epoch)
            .await
            .unwrap();
        if epoch == 0 {
            assert_eq!(
                total_processed_work, total_expected_work,
                "Not all items were processed."
            );
        } else {
            assert_eq!(
                total_processed_work, 0,
                "Not all items were processed in prior epoch."
            );
        }
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

pub async fn assert_trees_are_rollledover(
    pool: &SolanaRpcPool<SolanaRpcConnection>,
    state_tree_with_rollover_threshold_0: &Pubkey,
    address_tree_with_rollover_threshold_0: &Pubkey,
) {
    let rpc = pool.get_connection().await.unwrap();
    let address_merkle_tree = rpc
        .get_anchor_account::<AddressMerkleTreeAccount>(address_tree_with_rollover_threshold_0)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(
        address_merkle_tree
            .metadata
            .rollover_metadata
            .rolledover_slot,
        u64::MAX,
        "address_merkle_tree: {:?}",
        address_merkle_tree
    );
    let state_merkle_tree = rpc
        .get_anchor_account::<AddressMerkleTreeAccount>(state_tree_with_rollover_threshold_0)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(
        state_merkle_tree.metadata.rollover_metadata.rolledover_slot,
        u64::MAX,
        "state_merkle_tree: {:?}",
        state_merkle_tree
    );
}
async fn assert_foresters_registered(
    foresters: &[Pubkey],
    rpc: &SolanaRpcConnection,
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

#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
async fn test_epoch_double_registration() {
    println!("*****************************************************************");
    init(Some(LightValidatorConfig {
        enable_indexer: false,
        enable_prover: true,
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
        config.general_config.rpc_pool_size as u32,
    )
    .await
    .unwrap();

    let rpc = SolanaRpcConnection::new_with_retry(
        SolanaRpcUrl::Localnet,
        None,
        None,
        Some(forester_keypair.insecure_clone()),
    );

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
        &rpc,
        &env_accounts.governance_authority,
        &forester_keypair.pubkey(),
        light_registry::ForesterConfig::default(),
    )
    .await
    .unwrap();

    let indexer: TestIndexer<SolanaRpcConnection> =
        TestIndexer::init_from_env(&config.payer_keypair, &env_accounts, false, false).await;

    let indexer = Arc::new(indexer);

    for _ in 0..10 {
        let (shutdown_sender, shutdown_receiver) = oneshot::channel();
        let (work_report_sender, _work_report_receiver) = mpsc::channel(100);

        // Run the forester pipeline
        let service_handle = tokio::spawn(run_pipeline(
            config.clone(),
            indexer.clone(),
            shutdown_receiver,
            work_report_sender.clone(),
        ));

        sleep(Duration::from_secs(2)).await;

        shutdown_sender
            .send(())
            .expect("Failed to send shutdown signal");
        let result = service_handle.await.unwrap();
        assert!(result.is_ok(), "Registration should succeed");
    }

    let rpc = pool.get_connection().await.unwrap();
    let protocol_config = get_protocol_config(&*rpc).await;
    let solana_slot = rpc.get_slot().await.unwrap();
    let current_epoch = protocol_config.get_current_epoch(solana_slot);

    let forester_epoch_pda_address =
        get_forester_epoch_pda_from_authority(&config.payer_keypair.pubkey(), current_epoch).0;

    let forester_epoch_pda = rpc
        .get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_address)
        .await
        .unwrap();

    assert!(
        forester_epoch_pda.is_some(),
        "Forester should be registered"
    );
    let forester_epoch_pda = forester_epoch_pda.unwrap();
    assert_eq!(
        forester_epoch_pda.epoch, current_epoch,
        "Registered epoch should match current epoch"
    );
}
