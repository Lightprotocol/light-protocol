use anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL;
use env_logger::Env;
use forester::constants::SERVER_URL;
use forester::nullifier::{empty_address_queue, get_nullifier_queue, Config};
use forester::utils::spawn_test_validator;
use light_test_utils::e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig};
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::{get_test_env_accounts, REGISTRY_ID_TEST_KEYPAIR};
use log::info;
use solana_client::rpc_client::RpcClient;
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
        payer_keypair: env_accounts.governance_authority.insecure_clone(),
        concurrency_limit: 1,
        batch_size: 1,
        max_retries: 5,
    };

    let client = RpcClient::new(SERVER_URL);
    let signature = client
        .request_airdrop(&config.payer_keypair.pubkey(), LAMPORTS_PER_SOL * 1000)
        .unwrap();
    loop {
        let confirmed = client.confirm_transaction(&signature).unwrap();
        if confirmed {
            break;
        }
    }

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
        10,
        None,
        "../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;
    env.execute_rounds().await;
    assert_ne!(get_state_queue_length(&config).await, 0);
    info!(
        "Address merkle tree: nullifying queue of {} accounts...",
        get_state_queue_length(&config).await
    );

    let mut rpc = SolanaRpcConnection::new(Some(CommitmentConfig::confirmed())).await;
    let mut indexer = env.indexer;
    let _ = empty_address_queue(&mut rpc, &mut indexer, &config).await;
    assert_eq!(get_state_queue_length(&config).await, 0);
}

async fn get_state_queue_length(config: &Config) -> usize {
    let mut rpc = SolanaRpcConnection::new(None).await;
    let queue = get_nullifier_queue(&config.address_merkle_tree_queue_pubkey, &mut rpc)
        .await
        .unwrap();
    queue.len()
}
