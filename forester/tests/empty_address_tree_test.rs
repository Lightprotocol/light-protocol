use anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL;
use env_logger::Env;
use forester::external_services_config::ExternalServicesConfig;
use forester::nullifier::{empty_address_queue, get_nullifier_queue, Config};
use forester::utils::spawn_validator;
use light_test_utils::e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::solana_rpc::SolanaRpcUrl;
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::{get_test_env_accounts, REGISTRY_ID_TEST_KEYPAIR};
use log::info;
use solana_sdk::signature::{Keypair, Signer};
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn empty_address_tree_test() {
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
        KeypairActionConfig {
            compress_sol: Some(1.0),
            decompress_sol: Some(0.5),
            transfer_sol: Some(1.0),
            create_address: Some(1.0),
            compress_spl: None,
            decompress_spl: None,
            mint_spl: None,
            transfer_spl: None,
            max_output_accounts: Some(3),
        },
        GeneralActionConfig {
            add_keypair: Some(1.0),
            create_state_mt: Some(0.0),
            create_address_mt: Some(1.0),
            nullify_compressed_accounts: Some(0.0),
            empty_address_queue: Some(0.0),
        },
        0,
        None,
    )
    .await;
    for _ in 0..10 {
        env.create_address(None).await;
    }

    assert_ne!(get_address_queue_length(&config, &mut env.rpc).await, 0);
    info!(
        "Address merkle tree: nullifying queue of {} accounts...",
        get_address_queue_length(&config, &mut env.rpc).await
    );

    let mut rpc_clone = env.rpc.clone();
    let arc_indexer = Arc::new(tokio::sync::Mutex::new(env.indexer));
    let arc_rpc = Arc::new(tokio::sync::Mutex::new(env.rpc));
    let arc_config = Arc::new(config.clone());

    empty_address_queue(arc_indexer, arc_rpc, arc_config)
        .await
        .unwrap();
    assert_eq!(get_address_queue_length(&config, &mut rpc_clone).await, 0);
}

async fn get_address_queue_length<R: RpcConnection>(config: &Config, rpc: &mut R) -> usize {
    let queue = get_nullifier_queue(&config.address_merkle_tree_queue_pubkey, rpc)
        .await
        .unwrap();
    queue.len()
}
