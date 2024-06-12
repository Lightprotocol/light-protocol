use env_logger::Env;
use forester::constants::SERVER_URL;
use forester::nullifier::{get_nullifier_queue, nullify, Config};
use forester::utils::spawn_validator;
use light_test_utils::e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::{get_test_env_accounts, REGISTRY_ID_TEST_KEYPAIR};
use log::info;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::{Keypair, Signer};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_indexer() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    spawn_validator(Default::default()).await;
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

    let rpc = SolanaRpcConnection::new(None);

    rpc.client
        .request_airdrop(&rpc.get_payer().pubkey(), LAMPORTS_PER_SOL * 1000)
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(16)).await;

    let mut env = E2ETestEnv::<500, SolanaRpcConnection>::new(
        rpc,
        &env_accounts,
        KeypairActionConfig::test_forester_default(),
        GeneralActionConfig::test_forester_default(),
        0,
        None,
        "../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;

    let user_index = 0;
    let balance = env
        .rpc
        .get_balance(&env.users[user_index].keypair.pubkey())
        .await
        .unwrap();
    env.compress_sol(user_index, balance).await;
    for _ in 0..10 {
        env.transfer_sol(user_index).await;
    }

    assert_ne!(get_state_queue_length(&mut env.rpc, &config).await, 0);
    info!(
        "Nullifying queue of {} accounts...",
        get_state_queue_length(&mut env.rpc, &config).await
    );
    let _ = nullify(&mut env.indexer, &mut env.rpc, &config).await;
    assert_eq!(get_state_queue_length(&mut env.rpc, &config).await, 0);
}

async fn get_state_queue_length<R: RpcConnection>(rpc: &mut R, config: &Config) -> usize {
    let queue = get_nullifier_queue(&config.nullifier_queue_pubkey, rpc)
        .await
        .unwrap();
    queue.len()
}
