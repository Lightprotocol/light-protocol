use anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL;
use forester::{init_rpc, nullify_addresses};
use light_test_utils::e2e_test_env::E2ETestEnv;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::solana_rpc::SolanaRpcUrl;
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::get_test_env_accounts;
use log::info;
use solana_sdk::signature::Signer;
use std::sync::Arc;

mod test_utils;
use test_utils::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn empty_address_tree_test() {
    init(None).await;
    let env_accounts = get_test_env_accounts();
    let forester_config = forester_config();

    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);

    rpc.airdrop_lamports(&rpc.get_payer().pubkey(), LAMPORTS_PER_SOL * 100_000)
        .await
        .unwrap();

    let mut env = E2ETestEnv::<SolanaRpcConnection>::new(
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
