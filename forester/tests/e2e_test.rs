use std::sync::Arc;

use forester::rollover::RolloverState;
use forester::tree_sync::TreeData;
use forester::{get_state_queue_length, nullify_state, RpcPool};
use light_test_utils::e2e_test_env::E2ETestEnv;
use light_test_utils::indexer::TestIndexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::solana_rpc::SolanaRpcUrl;
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::get_test_env_accounts;
use log::info;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::Signer;
use tokio::time::sleep;

mod test_utils;
use test_utils::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_state_tree_nullifier() {
    init(None).await;
    let config = forester_config();
    let arc_config = Arc::new(config.clone());
    let pool = RpcPool::<SolanaRpcConnection>::new(arc_config.clone()).await;
    let env_accounts = get_test_env_accounts();

    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
    rpc.airdrop_lamports(&rpc.get_payer().pubkey(), LAMPORTS_PER_SOL * 100_000)
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
        info!("Round {} of {}", i, iterations);
        env.transfer_sol(user_index).await;
        sleep(std::time::Duration::from_secs(3)).await;
    }

    let state_trees: Vec<TreeData> = env
        .indexer
        .state_merkle_trees
        .iter()
        .map(|x| x.accounts.into())
        .collect();
    let rpc = pool.get_connection().await;
    for tree in state_trees {
        assert_ne!(get_state_queue_length(rpc.clone(), tree).await, 0);
        let arc_config = Arc::new(config.clone());
        let pool = RpcPool::<SolanaRpcConnection>::new(arc_config.clone()).await;
        let indexer = Arc::new(tokio::sync::Mutex::new(env.indexer.clone()));
        let state_rollover_state = Arc::new(RolloverState::new());
        nullify_state(
            arc_config.clone(),
            pool.clone(),
            indexer,
            tree,
            state_rollover_state,
        )
        .await;

        let rpc = pool.get_connection().await;
        assert_eq!(get_state_queue_length(rpc, tree).await, 0);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_1_all() {
    init(None).await;
    let env_accounts = get_test_env_accounts();
    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
    let config = forester_config();
    rpc.airdrop_lamports(&rpc.get_payer().pubkey(), LAMPORTS_PER_SOL * 100_000)
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
        1,
        None,
    )
    .await;
    env.execute_rounds().await;
}
