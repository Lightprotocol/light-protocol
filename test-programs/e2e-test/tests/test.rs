#![cfg(feature = "test-sbf")]

use light_test_utils::e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig};
use light_test_utils::rpc::{ProgramTestRpcConnection, SolanaRpcConnection};
use light_test_utils::test_env::{get_test_env_accounts, setup_test_programs_with_accounts};
use std::process::Command;

pub struct TestValidatorConfig {
    skip_indexer: bool,
    skip_prover: bool,
    setup_time: u64,
}
impl Default for TestValidatorConfig {
    fn default() -> Self {
        Self {
            skip_indexer: true,
            skip_prover: true,
            setup_time: 10,
        }
    }
}

// TODO: move into E2ETestEnv constructor
async fn spawn_test_validator(config: TestValidatorConfig) {
    println!("Starting validator...");

    let mut path = String::from("../../cli/test_bin/run test-validator");
    if config.skip_indexer {
        path += " --skip-indexer";
    }
    if config.skip_prover {
        path += " --skip-prover";
    }
    println!("Starting validator with path: {}", path);
    Command::new("sh")
        .arg("-c")
        .arg(path)
        .spawn()
        .expect("Failed to start server process");
    tokio::time::sleep(tokio::time::Duration::from_secs(config.setup_time)).await;
    println!("Validator started successfully");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_10_validator_all() {
    spawn_test_validator(TestValidatorConfig::default()).await;
    let env_accounts = get_test_env_accounts();
    let rpc = SolanaRpcConnection::new(None).await;
    let mut env = E2ETestEnv::<500, SolanaRpcConnection>::new(
        rpc,
        &env_accounts,
        KeypairActionConfig {
            max_output_accounts: Some(3),
            ..KeypairActionConfig::all_default()
        },
        GeneralActionConfig {
            nullify_compressed_accounts: Some(0.8),
            ..GeneralActionConfig::default()
        },
        10,
        None,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;
    env.execute_rounds().await;
}

#[tokio::test]
async fn test_10_all() {
    let (rpc, env_accounts) = setup_test_programs_with_accounts(None).await;
    let mut env = E2ETestEnv::<500, ProgramTestRpcConnection>::new(
        rpc,
        &env_accounts,
        KeypairActionConfig::all_default(),
        GeneralActionConfig::default(),
        10,
        None,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;
    env.execute_rounds().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_nullify() {
    spawn_test_validator(TestValidatorConfig {
        skip_indexer: true, // set to false to launch photon will crash though
        skip_prover: true,
        ..Default::default()
    })
    .await;
    let env_accounts = get_test_env_accounts();
    let rpc = SolanaRpcConnection::new(None).await;
    let mut env = E2ETestEnv::<500, SolanaRpcConnection>::new(
        rpc,
        &env_accounts,
        KeypairActionConfig {
            max_output_accounts: Some(3),
            ..KeypairActionConfig::all_default()
        },
        GeneralActionConfig {
            nullify_compressed_accounts: Some(1.0),
            empty_address_queue: Some(1.0),
            ..GeneralActionConfig::default()
        },
        0,
        // Seed for deterministic randomness, select None for a random seed
        Some(1),
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;
    // E2ETestEnv is instantiated with one user
    let user_index = 0;
    // If user has no spl balance it receives an airdrop
    env.transfer_spl(user_index).await;
    // Nullifies alls tx in queue with probability 1, also empties the queue with probability 1
    // (but there is nothing to do)
    env.activate_general_actions().await;
    // TODO: wait for photon to index
    env.transfer_spl(user_index).await;
    env.create_address().await;
    // TODO: wait for photon to index
    // Nullifies tx in queue with probability 1, and empties the queue with probability 1
    env.activate_general_actions().await;
    // TODO: wait for photon to index
    env.create_address().await;
}

//  cargo test-sbf -p e2e-test -- --nocapture --ignored --test test_10000_all > output.txt 2>&1
#[ignore]
#[tokio::test]
async fn test_10000_all() {
    // Will fail after inserting 500 addresses since the local indexed array is full
    // TODO: initialize the indexed array with heap memory so that the stack doesn't overflow with bigger size
    let (rpc, env_accounts) = setup_test_programs_with_accounts(None).await;
    let mut env = E2ETestEnv::<500, ProgramTestRpcConnection>::new(
        rpc,
        &env_accounts,
        KeypairActionConfig::all_default(),
        GeneralActionConfig::default(),
        10000,
        None,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;
    env.execute_rounds().await;
}
