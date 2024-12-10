#![cfg(feature = "test-sbf")]

use light_program_test::indexer::TestIndexer;
use light_program_test::test_env::setup_test_programs_with_accounts_with_protocol_config;
use light_program_test::test_rpc::ProgramTestRpcConnection;
use light_registry::protocol_config::state::ProtocolConfig;
use light_test_utils::e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig};

#[tokio::test]
async fn test_10_all() {
    let protocol_config = ProtocolConfig {
        genesis_slot: 0,
        slot_length: 100,
        registration_phase_length: 100,
        active_phase_length: 200,
        report_work_phase_length: 100,
        ..ProtocolConfig::default()
    };
    let (rpc, env_accounts) =
        setup_test_programs_with_accounts_with_protocol_config(None, protocol_config, true).await;

    let indexer: TestIndexer<ProgramTestRpcConnection> = TestIndexer::init_from_env(
        &env_accounts.forester.insecure_clone(),
        &env_accounts,
        Some(KeypairActionConfig::all_default().prover_config()),
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
    println!("stats {:?}", env.stats);
}

//  cargo test-sbf -p e2e-test -- --nocapture --ignored --test test_10000_all > output.txt 2>&1
#[ignore]
#[tokio::test]
async fn test_10000_all() {
    let protocol_config = ProtocolConfig {
        genesis_slot: 0,
        slot_length: 10,
        registration_phase_length: 100,
        active_phase_length: 200,
        report_work_phase_length: 100,
        ..ProtocolConfig::default()
    };
    let (rpc, env_accounts) =
        setup_test_programs_with_accounts_with_protocol_config(None, protocol_config, true).await;

    let indexer: TestIndexer<ProgramTestRpcConnection> = TestIndexer::init_from_env(
        &env_accounts.forester.insecure_clone(),
        &env_accounts,
        Some(KeypairActionConfig::all_default().prover_config()),
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
