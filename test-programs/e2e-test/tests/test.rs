#![cfg(feature = "test-sbf")]

use light_test_utils::e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig};

#[tokio::test]
async fn test_50_all() {
    let mut env = E2ETestEnv::<500>::new(
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
    let mut env = E2ETestEnv::<500>::new(
        KeypairActionConfig::all_default(),
        GeneralActionConfig::default(),
        10000,
        None,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;
    env.execute_rounds().await;
}
