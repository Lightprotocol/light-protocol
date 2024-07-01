#![cfg(feature = "test-sbf")]

use solana_sdk::signer::Signer;

use light_test_utils::e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig};
use light_test_utils::indexer::TestIndexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::ProgramTestRpcConnection;
use light_test_utils::test_env::setup_test_programs_with_accounts;

#[tokio::test]
async fn test_10_all() {
    let (rpc, env_accounts) = setup_test_programs_with_accounts(None).await;

    let indexer: TestIndexer<ProgramTestRpcConnection> = TestIndexer::init_from_env(
        &env_accounts.forester.insecure_clone(),
        &env_accounts,
        KeypairActionConfig::all_default().inclusion(),
        KeypairActionConfig::all_default().non_inclusion(),
    )
    .await;

    let mut env =
        E2ETestEnv::<ProgramTestRpcConnection, TestIndexer<ProgramTestRpcConnection>>::new(
        rpc,
            indexer,
        &env_accounts,
        KeypairActionConfig::all_default(),
        GeneralActionConfig::default(),
        10,
        None,
    )
    .await;
    env.execute_rounds().await;
}

//  cargo test-sbf -p e2e-test -- --nocapture --ignored --test test_10000_all > output.txt 2>&1
#[ignore]
#[tokio::test]
async fn test_10000_all() {
    // Will fail after inserting 500 addresses since the local indexed array is full
    // TODO: initialize the indexed array with heap memory so that the stack doesn't overflow with bigger size, write an indexed array vector abstraction for testing
    let (rpc, env_accounts) = setup_test_programs_with_accounts(None).await;

    let indexer: TestIndexer<ProgramTestRpcConnection> = TestIndexer::init_from_env(
        &env_accounts.forester.insecure_clone(),
        &env_accounts,
        KeypairActionConfig::all_default().inclusion(),
        KeypairActionConfig::all_default().non_inclusion(),
    )
    .await;

    let mut env =
        E2ETestEnv::<ProgramTestRpcConnection, TestIndexer<ProgramTestRpcConnection>>::new(
        rpc,
            indexer,
        &env_accounts,
        KeypairActionConfig::all_default_no_fee_assert(),
        GeneralActionConfig::test_with_rollover(),
        10000,
        None,
    )
    .await;
    env.execute_rounds().await;
}

#[ignore = "This is an example for the forester, remove once moved"]
#[tokio::test]
async fn test_address_tree_rollover() {
    // Will fail after inserting 500 addresses since the local indexed array is full
    // TODO: initialize the indexed array with heap memory so that the stack doesn't overflow with bigger size, write an indexed array vector abstraction for testing
    let (rpc, env_accounts) = setup_test_programs_with_accounts(None).await;

    let indexer: TestIndexer<ProgramTestRpcConnection> = TestIndexer::init_from_env(
        &env_accounts.forester.insecure_clone(),
        &env_accounts,
        KeypairActionConfig::all_default().inclusion(),
        KeypairActionConfig::all_default().non_inclusion(),
    )
    .await;

    let mut env =
        E2ETestEnv::<ProgramTestRpcConnection, TestIndexer<ProgramTestRpcConnection>>::new(
        rpc,
            indexer,
        &env_accounts,
        KeypairActionConfig::all_default_no_fee_assert(),
        GeneralActionConfig::default(),
        0,
        None,
    )
    .await;

    // remove address tree so that the address is created in the address that is
    // created next
    env.indexer.address_merkle_trees.remove(0);

    // create an address tree that is instantly ready for rollover
    env.create_address_tree(Some(0)).await;
    // create on transaction to fund the rollover fee
    env.create_address(None).await;
    // rollover address Merkle tree
    env.rollover_address_merkle_tree_and_queue(0).await.unwrap();
}

#[ignore = "This is an example for the forester, remove once moved"]
#[tokio::test]
async fn test_state_tree_rollover() {
    // Will fail after inserting 500 addresses since the local indexed array is full
    // TODO: initialize the indexed array with heap memory so that the stack doesn't overflow with bigger size, write an indexed array vector abstraction for testing
    let (rpc, env_accounts) = setup_test_programs_with_accounts(None).await;

    let indexer: TestIndexer<ProgramTestRpcConnection> = TestIndexer::init_from_env(
        &env_accounts.forester.insecure_clone(),
        &env_accounts,
        KeypairActionConfig::all_default().inclusion(),
        KeypairActionConfig::all_default().non_inclusion(),
    )
    .await;

    let mut env =
        E2ETestEnv::<ProgramTestRpcConnection, TestIndexer<ProgramTestRpcConnection>>::new(
        rpc,
            indexer,
        &env_accounts,
        KeypairActionConfig::all_default_no_fee_assert(),
        GeneralActionConfig::default(),
        0,
        None,
    )
    .await;

    // remove address tree so that the address is created in the address that is
    // created next
    env.indexer.state_merkle_trees.remove(0);

    // create an address tree that is instantly ready for rollover
    env.create_state_tree(Some(0)).await;
    let user_pubkey = env.users[0].keypair.pubkey();
    let user_balance = env.rpc.get_balance(&user_pubkey).await.unwrap();
    // create on transaction to fund the rollover fee
    env.compress_sol(0, user_balance).await;
    // rollover address Merkle tree
    env.rollover_state_merkle_tree_and_queue(0).await.unwrap();
}
