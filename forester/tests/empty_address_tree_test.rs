use env_logger::Env;
use forester::constants::SERVER_URL;
use forester::nullifier::{empty_address_queue, get_nullifier_queue, Config};
use forester::utils::{get_rpc_client_confirmed, spawn_test_validator};
use light_test_utils::e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig};
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::{get_test_env_accounts, REGISTRY_ID_TEST_KEYPAIR};
use log::info;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::{Keypair, Signer};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn empty_address_tree_test() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    spawn_test_validator().await;
    let env_accounts = get_test_env_accounts();
    let registry_keypair = Keypair::from_bytes(&REGISTRY_ID_TEST_KEYPAIR).unwrap();
    let config = Config {
        server_url: SERVER_URL.to_string(),
        nullifier_queue_pubkey: env_accounts.nullifier_queue_pubkey,
        state_merkle_tree_pubkey: env_accounts.merkle_tree_pubkey,
        address_merkle_tree_pubkey: env_accounts.address_merkle_tree_pubkey,
        address_merkle_tree_queue_pubkey: env_accounts.address_merkle_tree_queue_pubkey,
        registry_pubkey: registry_keypair.pubkey(),
        payer_keypair: env_accounts.forester.insecure_clone(),
        concurrency_limit: 1,
        batch_size: 1,
        max_retries: 5,
    };

    let rpc = SolanaRpcConnection::new(None).await;
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
        "../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;
    for _ in 0..10 {
        env.create_address().await;
    }

    assert_ne!(get_state_queue_length(&config), 0);
    info!(
        "Address merkle tree: nullifying queue of {} accounts...",
        get_state_queue_length(&config)
    );

    let mut rpc = SolanaRpcConnection::new(Some(CommitmentConfig::confirmed())).await;
    let mut indexer = env.indexer;
    empty_address_queue(&mut rpc, &mut indexer, &env_accounts.forester, &config)
        .await
        .unwrap();
    assert_eq!(get_state_queue_length(&config), 0);
}

fn get_state_queue_length(config: &Config) -> usize {
    let client = get_rpc_client_confirmed();
    let queue = get_nullifier_queue(&config.address_merkle_tree_queue_pubkey, &client).unwrap();
    queue.len()
}
