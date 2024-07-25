use forester::utils::LightValidatorConfig;
use light_test_utils::e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig};
use light_test_utils::indexer::TestIndexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::solana_rpc::SolanaRpcUrl;
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::get_test_env_accounts;
use log::info;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::Signer;
use test_utils::*;

mod test_utils;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_address_tree_rollover() {
    setup_logger();

    let validator_config = LightValidatorConfig {
        enable_forester: false,
        enable_prover: false,
        enable_indexer: false,
        wait_time: 25,
        ..LightValidatorConfig::default()
    };
    init(Some(validator_config)).await;
    let config = forester_config();
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

    // Will fail after inserting 500 addresses since the local indexed array is full
    // TODO: initialize the indexed array with heap memory so that the stack doesn't overflow with bigger size, write an indexed array vector abstraction for testing

    let mut env = E2ETestEnv::<SolanaRpcConnection, TestIndexer<SolanaRpcConnection>>::new(
        rpc,
        indexer,
        &env_accounts,
        KeypairActionConfig::all_default_no_fee_assert(),
        GeneralActionConfig::default(),
        1,
        None,
    )
    .await;

    info!("test_address_tree_rollover: env created");
    // remove address tree so that the address is created in the address that is
    // created next

    env.indexer.address_merkle_trees.remove(0);
    info!("test_address_tree_rollover: removed address tree");

    // create an address tree that is instantly ready for rollover
    env.create_address_tree(Some(0)).await;
    info!("test_address_tree_rollover: created address tree");

    // create on transaction to fund the rollover fee
    env.create_address(None).await;
    info!("test_address_tree_rollover: created address");

    // rollover address Merkle tree
    env.rollover_address_merkle_tree_and_queue(0).await.unwrap();
    info!("test_address_tree_rollover: rollover address tree");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_state_tree_rollover() {
    let validator_config = LightValidatorConfig {
        enable_forester: false,
        enable_prover: false,
        enable_indexer: false,
        wait_time: 25,
        ..LightValidatorConfig::default()
    };
    init(Some(validator_config)).await;

    let config = forester_config();
    let env_accounts = get_test_env_accounts();

    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
    rpc.airdrop_lamports(&rpc.get_payer().pubkey(), LAMPORTS_PER_SOL * 10000)
        .await
        .unwrap();

    let indexer: TestIndexer<SolanaRpcConnection> = TestIndexer::init_from_env(
        &config.payer_keypair,
        &env_accounts,
        keypair_action_config().inclusion(),
        keypair_action_config().non_inclusion(),
    )
    .await;

    // Will fail after inserting 500 addresses since the local indexed array is full
    // TODO: initialize the indexed array with heap memory so that the stack doesn't overflow with bigger size, write an indexed array vector abstraction for testing

    let mut env = E2ETestEnv::<SolanaRpcConnection, TestIndexer<SolanaRpcConnection>>::new(
        rpc,
        indexer,
        &env_accounts,
        KeypairActionConfig::all_default_no_fee_assert(),
        GeneralActionConfig::default(),
        1,
        None,
    )
    .await;

    // remove address tree so that the address is created in the address that is
    // created next
    info!(
        "address_merkle_trees len: {}",
        env.indexer.address_merkle_trees.len()
    );

    env.indexer.state_merkle_trees.remove(0);
    env.create_state_tree(Some(0)).await;

    let payer_keypair = env.rpc.get_payer().insecure_clone();

    for i in 0..5 {
        env.compress_sol_deterministic(&payer_keypair, LAMPORTS_PER_SOL, Some(i))
            .await;
        env.rollover_state_merkle_tree_and_queue(i).await.unwrap();
    }
}
