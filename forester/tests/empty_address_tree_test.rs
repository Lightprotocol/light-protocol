use anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL;
use forester::external_services_config::ExternalServicesConfig;
use forester::nullifier::state::get_nullifier_queue;
use forester::utils::{spawn_validator, LightValidatorConfig};
use forester::{init_rpc, nullify_addresses, ForesterConfig};
use light_test_utils::e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::solana_rpc::SolanaRpcUrl;
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::{get_test_env_accounts, REGISTRY_ID_TEST_KEYPAIR};
use log::{info, LevelFilter};
use solana_sdk::signature::{Keypair, Signer};
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn empty_address_tree_test() {
    setup_logger();
    spawn_test_validator().await;
    let env_accounts = get_test_env_accounts();
    let forester_config = setup_forester();

    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);

    rpc.airdrop_lamports(&rpc.get_payer().pubkey(), LAMPORTS_PER_SOL * 100_000)
        .await
        .unwrap();

    let mut env = E2ETestEnv::<500, SolanaRpcConnection>::new(
        rpc,
        &env_accounts,
        keypair_action_config(),
        general_action_config(),
        0,
        None,
    )
    .await;

    let config = Arc::new(forester_config.clone());
    let rpc = init_rpc(&config, true).await;
    let rpc = Arc::new(tokio::sync::Mutex::new(rpc));

    let indexer = Arc::new(tokio::sync::Mutex::new(env.indexer.clone()));

    for _ in 0..10 {
        env.create_address(None).await;
    }

    assert_ne!(get_address_queue_length(&config, &mut env.rpc).await, 0);
    info!(
        "Address merkle tree: nullifying queue of {} accounts...",
        get_address_queue_length(&config, &mut env.rpc).await
    );

    nullify_addresses(config.clone(), rpc, indexer).await;
    assert_eq!(get_address_queue_length(&config, &mut env.rpc).await, 0);
}

fn setup_logger() {
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(LevelFilter::Info.to_string()),
    )
    .is_test(true)
    .try_init();
}

async fn spawn_test_validator() {
    let config = LightValidatorConfig {
        enable_indexer: true,
        ..LightValidatorConfig::default()
    };
    spawn_validator(config).await;
}

fn keypair_action_config() -> KeypairActionConfig {
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
    }
}

fn general_action_config() -> GeneralActionConfig {
    GeneralActionConfig {
        add_keypair: Some(1.0),
        create_state_mt: Some(1.0),
        create_address_mt: Some(1.0),
        nullify_compressed_accounts: Some(1.0),
        empty_address_queue: Some(1.0),
        rollover: None,
    }
}

fn setup_forester() -> ForesterConfig {
    let env_accounts = get_test_env_accounts();
    let registry_keypair = Keypair::from_bytes(&REGISTRY_ID_TEST_KEYPAIR).unwrap();
    ForesterConfig {
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
        max_concurrent_batches: 5,
    }
}
async fn get_address_queue_length<R: RpcConnection>(config: &ForesterConfig, rpc: &mut R) -> usize {
    let queue = get_nullifier_queue(&config.address_merkle_tree_queue_pubkey, rpc)
        .await
        .unwrap();
    queue.len()
}
