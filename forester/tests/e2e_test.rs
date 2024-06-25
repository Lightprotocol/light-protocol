use env_logger::Env;
use forester::external_services_config::ExternalServicesConfig;
use forester::nullifier::{get_nullifier_queue, nullify, Config};
use forester::utils::spawn_validator;
use light_test_utils::e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::solana_rpc::SolanaRpcUrl;
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::{get_test_env_accounts, REGISTRY_ID_TEST_KEYPAIR};
use log::info;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::{Keypair, Signer};
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_state_tree_nullifier() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    spawn_validator(Default::default()).await;
    let env_accounts = get_test_env_accounts();
    let registry_keypair = Keypair::from_bytes(&REGISTRY_ID_TEST_KEYPAIR).unwrap();
    let config = Config {
        external_services: ExternalServicesConfig::local(),
        nullifier_queue_pubkey: env_accounts.nullifier_queue_pubkey,
        state_merkle_tree_pubkey: env_accounts.merkle_tree_pubkey,
        address_merkle_tree_pubkey: env_accounts.address_merkle_tree_pubkey,
        address_merkle_tree_queue_pubkey: env_accounts.address_merkle_tree_queue_pubkey,
        registry_pubkey: registry_keypair.pubkey(),
        payer_keypair: env_accounts.forester.insecure_clone(),
        concurrency_limit: 1,
        batch_size: 1,
        max_retries: 5,
        max_concurrent_batches: 5,
    };

    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);

    rpc.airdrop_lamports(&rpc.get_payer().pubkey(), LAMPORTS_PER_SOL * 1000)
        .await
        .unwrap();

    let mut env = E2ETestEnv::<500, SolanaRpcConnection>::new(
        rpc,
        &env_accounts,
        KeypairActionConfig::test_forester_default(),
        GeneralActionConfig::test_forester_default(),
        0,
        None,
    )
    .await;

    let user_index = 0;
    let balance = env
        .rpc
        .get_balance(&env.users[user_index].keypair.pubkey())
        .await
        .unwrap();
    env.compress_sol(user_index, balance).await;
    let iterations = 100;
    for i in 0..iterations {
        info!("Round {} of {}", i, iterations);
        env.transfer_sol(user_index).await;
    }

    assert_ne!(get_state_queue_length(&mut env.rpc, &config).await, 0);
    info!(
        "Nullifying queue of {} accounts...",
        get_state_queue_length(&mut env.rpc, &config).await
    );

    let indexer = Arc::new(tokio::sync::Mutex::new(env.indexer));
    let rpc = Arc::new(tokio::sync::Mutex::new(env.rpc));
    let config = Arc::new(config);
    nullify(indexer, rpc, config).await.unwrap();
    // assert_eq!(get_state_queue_length(&mut *rpc.lock().await, &config).await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_1_all() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    spawn_validator(Default::default()).await;
    let env_accounts = get_test_env_accounts();
    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);

    rpc.airdrop_lamports(&rpc.get_payer().pubkey(), LAMPORTS_PER_SOL * 1000)
        .await
        .unwrap();

    let mut env = E2ETestEnv::<500, SolanaRpcConnection>::new(
        rpc,
        &env_accounts,
        KeypairActionConfig {
            compress_sol: Some(1.0),
            decompress_sol: Some(1.0),
            transfer_sol: Some(1.0),
            create_address: Some(1.0),
            compress_spl: Some(1.0),
            decompress_spl: Some(1.0),
            mint_spl: Some(1.0),
            transfer_spl: Some(1.0),
            max_output_accounts: Some(3),
        },
        GeneralActionConfig {
            add_keypair: Some(1.0),
            create_state_mt: Some(1.0),
            create_address_mt: Some(1.0),
            nullify_compressed_accounts: Some(1.0),
            empty_address_queue: Some(1.0),
        },
        1,
        None,
    )
    .await;
    env.execute_rounds().await;
}

async fn get_state_queue_length<R: RpcConnection>(rpc: &mut R, config: &Config) -> usize {
    let queue = get_nullifier_queue(&config.nullifier_queue_pubkey, rpc)
        .await
        .unwrap();
    queue.len()
}
