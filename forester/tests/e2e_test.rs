use forester::epoch_manager::fetch_queue_data;
use forester::utils::LightValidatorConfig;
use forester::{run_pipeline, RpcPool};
use light_test_utils::e2e_test_env::E2ETestEnv;
use light_test_utils::indexer::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts, TestIndexer};
use light_test_utils::registry::register_test_forester;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::solana_rpc::SolanaRpcUrl;
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::get_test_env_accounts;
use log::debug;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::sleep;

mod test_utils;
use test_utils::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_epoch_monitor_with_test_indexer_and_1_forester() {
    init(Some(LightValidatorConfig {
        enable_indexer: false,
        enable_prover: true,
        enable_forester: false,
        ..LightValidatorConfig::default()
    }))
    .await;
    let forester_keypair = Keypair::new();

    let mut env_accounts = get_test_env_accounts();
    env_accounts.forester = forester_keypair.insecure_clone();

    let mut config = forester_config();
    config.payer_keypair = forester_keypair.insecure_clone();

    let config = Arc::new(config);
    let pool = RpcPool::<SolanaRpcConnection>::new(config.clone()).await;

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

    let indexer: TestIndexer<SolanaRpcConnection> = TestIndexer::init_from_env(
        &config.payer_keypair,
        &env_accounts,
        keypair_action_config().inclusion(),
        keypair_action_config().non_inclusion(),
    )
    .await;

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
    let iterations = 10;

    for i in 0..iterations {
        debug!("Round {} of {}", i, iterations);
        env.transfer_sol(user_index).await;
        sleep(std::time::Duration::from_millis(100)).await;
    }

    for _ in 0..iterations {
        env.create_address(None).await;
    }
    let state_trees: Vec<StateMerkleTreeAccounts> = env
        .indexer
        .state_merkle_trees
        .iter()
        .map(|x| x.accounts)
        .collect();

    for tree in state_trees.iter() {
        let queue_length = fetch_queue_data(pool.get_connection().await, &tree.nullifier_queue)
            .await
            .unwrap()
            .len();
        assert_ne!(queue_length, 0);
    }

    let address_trees: Vec<AddressMerkleTreeAccounts> = env
        .indexer
        .address_merkle_trees
        .iter()
        .map(|x| x.accounts)
        .collect();
    for tree in address_trees.iter() {
        let queue_length = fetch_queue_data(pool.get_connection().await, &tree.queue)
            .await
            .unwrap()
            .len();
        assert_ne!(queue_length, 0);
    }

    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    let (work_report_sender, mut work_report_receiver) = mpsc::channel(100);

    let service_handle = tokio::spawn(run_pipeline(
        config.clone(),
        Arc::new(Mutex::new(env.indexer)),
        shutdown_receiver,
        work_report_sender,
    ));

    let mut total_processed = 0;
    if let Some(report) = work_report_receiver.recv().await {
        total_processed += report.processed_items;
    }

    for tree in state_trees {
        let queue_length = fetch_queue_data(pool.get_connection().await, &tree.nullifier_queue)
            .await
            .unwrap()
            .len();
        assert_eq!(queue_length, 0);
    }

    for tree in address_trees {
        let queue_length = fetch_queue_data(pool.get_connection().await, &tree.queue)
            .await
            .unwrap()
            .len();
        assert_eq!(queue_length, 0);
    }

    assert!(total_processed > 0, "No items were processed");

    shutdown_sender
        .send(())
        .expect("Failed to send shutdown signal");
    service_handle.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn test_epoch_monitor_with_2_foresters() {
    init(None).await;
    let forester_keypair1 = Keypair::new();
    let forester_keypair2 = Keypair::new();

    let mut env_accounts = get_test_env_accounts();
    env_accounts.forester = forester_keypair1.insecure_clone();

    let mut config1 = forester_config();
    config1.payer_keypair = forester_keypair1.insecure_clone();
    let config1 = Arc::new(config1);

    let mut config2 = forester_config();
    config2.payer_keypair = forester_keypair2.insecure_clone();
    let config2 = Arc::new(config2);

    let pool = RpcPool::<SolanaRpcConnection>::new(config1.clone()).await;

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

    let indexer: TestIndexer<SolanaRpcConnection> = TestIndexer::init_from_env(
        &config1.payer_keypair,
        &env_accounts,
        keypair_action_config().inclusion(),
        keypair_action_config().non_inclusion(),
    )
    .await;

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
    let iterations = 10;
    for i in 0..iterations {
        debug!("Round {} of {}", i, iterations);
        env.transfer_sol(user_index).await;
        sleep(std::time::Duration::from_millis(100)).await;
    }

    let state_trees: Vec<StateMerkleTreeAccounts> = env
        .indexer
        .state_merkle_trees
        .iter()
        .map(|x| x.accounts)
        .collect();

    for tree in state_trees.iter() {
        let queue_length = fetch_queue_data(pool.get_connection().await, &tree.nullifier_queue)
            .await
            .unwrap()
            .len();
        assert_ne!(queue_length, 0);
    }

    let address_trees: Vec<AddressMerkleTreeAccounts> = env
        .indexer
        .address_merkle_trees
        .iter()
        .map(|x| x.accounts)
        .collect();
    for tree in address_trees.iter() {
        let queue_length = fetch_queue_data(pool.get_connection().await, &tree.queue)
            .await
            .unwrap()
            .len();
        assert_ne!(queue_length, 0);
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

    let mut total_processed = 0;
    for _ in 0..2 {
        if let Some(report) = work_report_receiver1.recv().await {
            total_processed += report.processed_items;
        }
        if let Some(report) = work_report_receiver2.recv().await {
            total_processed += report.processed_items;
        }
    }

    for tree in state_trees {
        let queue_length = fetch_queue_data(pool.get_connection().await, &tree.nullifier_queue)
            .await
            .unwrap()
            .len();
        assert_eq!(queue_length, 0);
    }

    for tree in address_trees {
        let queue_length = fetch_queue_data(pool.get_connection().await, &tree.queue)
            .await
            .unwrap()
            .len();
        assert_eq!(queue_length, 0);
    }

    assert!(total_processed > 0, "No items were processed");

    shutdown_sender1
        .send(())
        .expect("Failed to send shutdown signal to forester 1");
    shutdown_sender2
        .send(())
        .expect("Failed to send shutdown signal to forester 2");
    service_handle1.await.unwrap().unwrap();
    service_handle2.await.unwrap().unwrap();
}
