use std::sync::Arc;

use anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::Signer;

use forester::rollover::RolloverState;
use forester::tree_sync::TreeData;
use forester::{get_address_queue_length, nullify_addresses, RpcPool};
use light_test_utils::e2e_test_env::E2ETestEnv;
use light_test_utils::indexer::TestIndexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::solana_rpc::SolanaRpcUrl;
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::get_test_env_accounts;
use test_utils::*;

mod test_utils;
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn empty_address_tree_test() {
    init(None).await;
    let env_accounts = get_test_env_accounts();
    let forester_config = forester_config();

    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);

    rpc.airdrop_lamports(&rpc.get_payer().pubkey(), LAMPORTS_PER_SOL * 100_000)
        .await
        .unwrap();

    let indexer: TestIndexer<SolanaRpcConnection> = TestIndexer::init_from_env(
        &forester_config.payer_keypair,
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
        None,
    )
    .await;

    let config = Arc::new(forester_config);
    let pool = RpcPool::<SolanaRpcConnection>::new(config.clone()).await;
    let indexer = Arc::new(tokio::sync::Mutex::new(env.indexer.clone()));

    for _ in 0..10 {
        env.create_address(None).await;
    }

    let address_trees: Vec<TreeData> = env
        .indexer
        .address_merkle_trees
        .iter()
        .map(|x| x.accounts.into())
        .collect();
    for tree in address_trees {
        let rpc = pool.get_connection().await;
        assert_ne!(get_address_queue_length(rpc, tree).await, 0);
        let rollover_state = Arc::new(RolloverState::new());
        nullify_addresses(
            config.clone(),
            pool.clone(),
            indexer.clone(),
            tree,
            rollover_state,
        )
        .await;
        let rpc = pool.get_connection().await;
        assert_eq!(get_address_queue_length(rpc, tree).await, 0);
    }
}
