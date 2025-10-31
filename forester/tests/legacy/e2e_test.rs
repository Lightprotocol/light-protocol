use std::{collections::HashSet, sync::Arc, time::Duration};

use account_compression::{
    utils::constants::{ADDRESS_QUEUE_VALUES, STATE_NULLIFIER_QUEUE_VALUES},
    AddressMerkleTreeAccount,
};
use forester::{queue_helpers::fetch_queue_item_data, run_pipeline, utils::get_protocol_config};
use forester_utils::{
    registry::register_test_forester,
    rpc_pool::{SolanaRpcPool, SolanaRpcPoolBuilder},
};
use light_client::{
    indexer::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts},
    local_test_validator::LightValidatorConfig,
    rpc::{client::RpcUrl, LightClient, LightClientConfig, Rpc, RpcError},
};
use light_program_test::{accounts::test_accounts::TestAccounts, indexer::TestIndexer};
use light_registry::{utils::get_forester_epoch_pda_from_authority, ForesterEpochPda};
use light_test_utils::{e2e_test_env::E2ETestEnv, update_test_forester};
use serial_test::serial;
use solana_sdk::{
    commitment_config::CommitmentConfig, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey,
    signature::Keypair, signer::Signer,
};
use tokio::{
    sync::{mpsc, oneshot, Mutex},
    time::sleep,
};

mod test_utils;
use test_utils::*;

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
async fn test_epoch_monitor_with_2_foresters() {
    init(Some(LightValidatorConfig {
        enable_indexer: false,
        enable_prover: true,
        wait_time: 90,
        sbf_programs: vec![],
        limit_ledger_size: None,
        grpc_port: None,
    }))
    .await;
    let forester_keypair1 = Keypair::new();
    let forester_keypair2 = Keypair::new();

    let mut test_accounts = TestAccounts::get_local_test_validator_accounts();
    test_accounts.protocol.forester = forester_keypair1.insecure_clone();

    let mut config1 = forester_config();
    config1.payer_keypair = forester_keypair1.insecure_clone();

    let mut config2 = forester_config();
    config2.payer_keypair = forester_keypair2.insecure_clone();

    let pool = SolanaRpcPoolBuilder::<LightClient>::default()
        .url(config1.external_services.rpc_url.to_string())
        .commitment(CommitmentConfig::confirmed())
        .build()
        .await
        .unwrap();

    let mut rpc = LightClient::new(LightClientConfig::local_no_indexer())
        .await
        .unwrap();
    rpc.payer = forester_keypair1.insecure_clone();

    // Airdrop to both foresters and governance authority
    for keypair in [
        &forester_keypair1,
        &forester_keypair2,
        &test_accounts.protocol.governance_authority,
    ] {
        rpc.airdrop_lamports(&keypair.pubkey(), LAMPORTS_PER_SOL * 100_000)
            .await
            .unwrap();
    }

    // Register both foresters
    for forester_keypair in [&forester_keypair1, &forester_keypair2] {
        register_test_forester(
            &mut rpc,
            &test_accounts.protocol.governance_authority,
            &forester_keypair.pubkey(),
            light_registry::ForesterConfig::default(),
        )
        .await
        .unwrap();
    }

    let new_forester_keypair1 = Keypair::new();
    let new_forester_keypair2 = Keypair::new();

    for forester_keypair in [&new_forester_keypair1, &new_forester_keypair2] {
        rpc.airdrop_lamports(&forester_keypair.pubkey(), LAMPORTS_PER_SOL * 100_000)
            .await
            .unwrap();
    }

    update_test_forester(
        &mut rpc,
        &forester_keypair1,
        &forester_keypair1.pubkey(),
        Some(&new_forester_keypair1),
        light_registry::ForesterConfig::default(),
    )
    .await
    .unwrap();

    update_test_forester(
        &mut rpc,
        &forester_keypair2,
        &forester_keypair2.pubkey(),
        Some(&new_forester_keypair2),
        light_registry::ForesterConfig::default(),
    )
    .await
    .unwrap();

    config1.derivation_pubkey = forester_keypair1.pubkey();
    config1.payer_keypair = new_forester_keypair1.insecure_clone();

    config2.derivation_pubkey = forester_keypair2.pubkey();
    config2.payer_keypair = new_forester_keypair2.insecure_clone();

    let config1 = Arc::new(config1);
    let config2 = Arc::new(config2);

    let indexer: TestIndexer =
        TestIndexer::init_from_acounts(&config1.payer_keypair, &test_accounts, 0).await;

    let mut env = E2ETestEnv::<LightClient, TestIndexer>::new(
        rpc,
        indexer,
        &test_accounts,
        keypair_action_config(),
        general_action_config(),
        0,
        Some(0),
    )
    .await;
    // removing batched Merkle tree
    env.indexer.state_merkle_trees.remove(1);
    // removing batched address tree
    env.indexer.address_merkle_trees.remove(1);
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
    let state_tree_with_rollover_threshold_0 =
        env.indexer.state_merkle_trees[1].accounts.merkle_tree;
    let address_tree_with_rollover_threshold_0 =
        env.indexer.address_merkle_trees[1].accounts.merkle_tree;

    println!(
        "State tree with rollover threshold 0: {:?}",
        state_tree_with_rollover_threshold_0
    );
    println!(
        "Address tree with rollover threshold 0: {:?}",
        address_tree_with_rollover_threshold_0
    );

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

    let indexer = Arc::new(Mutex::new(env.indexer));

    let service_handle1 = tokio::spawn(run_pipeline::<LightClient, TestIndexer>(
        config1.clone(),
        None,
        None,
        indexer.clone(),
        shutdown_receiver1,
        work_report_sender1,
    ));
    let service_handle2 = tokio::spawn(run_pipeline::<LightClient, TestIndexer>(
        config2.clone(),
        None,
        None,
        indexer,
        shutdown_receiver2,
        work_report_sender2,
    ));

    const EXPECTED_EPOCHS: u64 = 3; // We expect to process 2 epochs (0 and 1)

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
    // assert!(processed_epochs.contains(&0), "Epoch 0 was not processed");
    assert!(processed_epochs.contains(&1), "Epoch 1 was not processed");

    assert_trees_are_rolledover(
        &pool,
        &state_tree_with_rollover_threshold_0,
        &address_tree_with_rollover_threshold_0,
    )
    .await;
    // assert queues have been emptied
    assert_queue_len(&pool, &state_trees, &address_trees, &mut 0, 0, false).await;
    let mut rpc = pool.get_connection().await.unwrap();
    let forester_pubkeys = [config1.derivation_pubkey, config2.derivation_pubkey];

    // assert that foresters registered for epoch 1 and 2 (no new work is emitted after epoch 0)
    // Assert that foresters have registered all processed epochs and the next epoch (+1)
    for epoch in 1..=EXPECTED_EPOCHS {
        let total_processed_work =
            assert_foresters_registered(&forester_pubkeys[..], &mut rpc, epoch)
                .await
                .unwrap();
        if epoch == 1 {
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
pub async fn assert_trees_are_rolledover(
    pool: &SolanaRpcPool<LightClient>,
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
    rpc: &mut LightClient,
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

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
async fn test_epoch_double_registration() {
    println!("*****************************************************************");

    init(Some(LightValidatorConfig {
        enable_indexer: false,
        enable_prover: true,
        wait_time: 90,
        sbf_programs: vec![],
        limit_ledger_size: None,
        grpc_port: None,
    }))
    .await;

    let forester_keypair = Keypair::new();

    let mut test_accounts = TestAccounts::get_local_test_validator_accounts();
    test_accounts.protocol.forester = forester_keypair.insecure_clone();

    let mut config = forester_config();
    config.payer_keypair = forester_keypair.insecure_clone();

    let pool = SolanaRpcPoolBuilder::<LightClient>::default()
        .url(config.external_services.rpc_url.to_string())
        .commitment(CommitmentConfig::confirmed())
        .build()
        .await
        .unwrap();

    let mut rpc = LightClient::new(LightClientConfig {
        url: RpcUrl::Localnet.to_string(),
        photon_url: None,
        commitment_config: Some(CommitmentConfig::confirmed()),
        fetch_active_tree: false,
    })
    .await
    .unwrap();
    rpc.payer = forester_keypair.insecure_clone();

    rpc.airdrop_lamports(&forester_keypair.pubkey(), LAMPORTS_PER_SOL * 100_000)
        .await
        .unwrap();

    rpc.airdrop_lamports(
        &test_accounts.protocol.governance_authority.pubkey(),
        LAMPORTS_PER_SOL * 100_000,
    )
    .await
    .unwrap();

    register_test_forester(
        &mut rpc,
        &test_accounts.protocol.governance_authority,
        &forester_keypair.pubkey(),
        light_registry::ForesterConfig::default(),
    )
    .await
    .unwrap();

    let new_forester_keypair = Keypair::new();

    rpc.airdrop_lamports(&new_forester_keypair.pubkey(), LAMPORTS_PER_SOL * 100_000)
        .await
        .unwrap();
    update_test_forester(
        &mut rpc,
        &forester_keypair,
        &forester_keypair.pubkey(),
        Some(&new_forester_keypair),
        light_registry::ForesterConfig::default(),
    )
    .await
    .unwrap();

    config.derivation_pubkey = forester_keypair.pubkey();
    config.payer_keypair = new_forester_keypair.insecure_clone();

    let config = Arc::new(config);

    let mut indexer: TestIndexer =
        TestIndexer::init_from_acounts(&config.payer_keypair, &test_accounts, 0).await;
    indexer.state_merkle_trees.remove(1);
    let indexer = Arc::new(Mutex::new(indexer));

    for _ in 0..10 {
        let (shutdown_sender, shutdown_receiver) = oneshot::channel();
        let (work_report_sender, _work_report_receiver) = mpsc::channel(100);

        // Run the forester pipeline
        let service_handle = tokio::spawn(run_pipeline::<LightClient, TestIndexer>(
            config.clone(),
            None,
            None,
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

    let mut rpc = pool.get_connection().await.unwrap();
    let protocol_config = get_protocol_config(&mut *rpc).await;
    let solana_slot = rpc.get_slot().await.unwrap();
    let current_epoch = protocol_config.get_current_epoch(solana_slot);

    let forester_epoch_pda_address =
        get_forester_epoch_pda_from_authority(&config.derivation_pubkey, current_epoch).0;

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

pub async fn assert_queue_len(
    pool: &SolanaRpcPool<LightClient>,
    state_trees: &[StateMerkleTreeAccounts],
    address_trees: &[AddressMerkleTreeAccounts],
    total_expected_work: &mut u64,
    expected_len: usize,
    not_empty: bool,
) {
    for tree in state_trees.iter() {
        let mut rpc = pool.get_connection().await.unwrap();
        let queue_length = fetch_queue_item_data(
            &mut *rpc,
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
        let mut rpc = pool.get_connection().await.unwrap();
        let queue_length = fetch_queue_item_data(
            &mut *rpc,
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
