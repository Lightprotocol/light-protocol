#![cfg(feature = "test-sbf")]

use light_test_utils::e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig};
use light_test_utils::rpc::{ProgramTestRpcConnection, SolanaRpcConnection};
use light_test_utils::test_env::{get_test_env_accounts, setup_test_programs_with_accounts};
use std::process::Command;

async fn spawn_test_validator() {
    println!("Starting validator...");
    let path = "../../cli/test_bin/run test-validator --skip-indexer --skip-prover";
    Command::new("sh")
        .arg("-c")
        .arg(path)
        .spawn()
        .expect("Failed to start server process");
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    println!("Validator started successfully");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_10_validator_all() {
    spawn_test_validator().await;
    let env_accounts = get_test_env_accounts();
    let rpc = SolanaRpcConnection::new().await;
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
