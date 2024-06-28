use std::sync::Arc;

use forester::external_services_config::ExternalServicesConfig;
use forester::nullifier::state::get_nullifier_queue;
use forester::utils::{spawn_validator, LightValidatorConfig};
use forester::{nullify_state, ForesterConfig};
use light_test_utils::e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::solana_rpc::SolanaRpcUrl;
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::{get_test_env_accounts, REGISTRY_ID_TEST_KEYPAIR};
use log::{info, LevelFilter};
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::{Keypair, Signer};
use tokio::time::sleep;

async fn init() {
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(LevelFilter::Info.to_string()),
    )
    .is_test(true)
    .try_init();

    let config = LightValidatorConfig {
        enable_indexer: true,
        ..LightValidatorConfig::default()
    };
    spawn_validator(config).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_state_tree_nullifier() {
    init().await;
    info!("Starting test_state_tree_nullifier");
    let env_accounts = get_test_env_accounts();
    let registry_keypair = Keypair::from_bytes(&REGISTRY_ID_TEST_KEYPAIR).unwrap();
    let config = ForesterConfig {
        external_services: ExternalServicesConfig {
            rpc_url: "http://localhost:8899".to_string(),
            ws_rpc_url: "ws://localhost:8900".to_string(),
            indexer_url: "http://localhost:8784".to_string(),
            prover_url: "http://localhost:3001".to_string(),
            derivation: "En9a97stB3Ek2n6Ey3NJwCUJnmTzLMMEA5C69upGDuQP".to_string(),
        },
        nullifier_queue_pubkey: env_accounts.nullifier_queue_pubkey,
        state_merkle_tree_pubkey: env_accounts.merkle_tree_pubkey,
        address_merkle_tree_pubkey: env_accounts.address_merkle_tree_pubkey,
        address_merkle_tree_queue_pubkey: env_accounts.address_merkle_tree_queue_pubkey,
        registry_pubkey: registry_keypair.pubkey(),
        payer_keypair: env_accounts.forester.insecure_clone(),
        concurrency_limit: 1,
        batch_size: 1,
        max_retries: 5,
        max_concurrent_batches: 1,
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
        Some(6214032178617709141),
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
        info!("Round {} of {}", i, iterations);
        env.transfer_sol(user_index).await;
        sleep(std::time::Duration::from_secs(3)).await;
    }

    assert_ne!(get_state_queue_length(&mut env.rpc, &config).await, 0);
    info!(
        "Nullifying queue of {} accounts...",
        get_state_queue_length(&mut env.rpc, &config).await
    );

    let arc_config = Arc::new(config.clone());
    nullify_state(arc_config).await;
    assert_eq!(get_state_queue_length(&mut env.rpc, &config).await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_1_all() {
    init().await;
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
            fee_assert: true,
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

async fn get_state_queue_length<R: RpcConnection>(rpc: &mut R, config: &ForesterConfig) -> usize {
    let queue = get_nullifier_queue(&config.nullifier_queue_pubkey, rpc)
        .await
        .unwrap();
    queue.len()
}
