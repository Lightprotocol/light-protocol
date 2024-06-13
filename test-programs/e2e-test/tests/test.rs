#![cfg(feature = "test-sbf")]

use light_registry::protocol_config::state::ProtocolConfig;
use light_test_utils::e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig};
use light_test_utils::indexer::TestIndexer;
use light_test_utils::rpc::ProgramTestRpcConnection;
use light_test_utils::test_env::{
    set_env_with_delegate_and_forester, setup_test_programs_with_accounts_with_protocol_config,
};

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
    // let (rpc, env_accounts) =
    //     setup_test_programs_with_accounts_with_protocol_config(None, protocol_config, true).await;

    // let indexer: TestIndexer<ProgramTestRpcConnection> = TestIndexer::init_from_env(
    //     &env_accounts.forester.insecure_clone(),
    //     &env_accounts,
    //     KeypairActionConfig::all_default().inclusion(),
    //     KeypairActionConfig::all_default().non_inclusion(),
    // )
    // .await;
    let (mut e2e_env, _delegate_keypair, _env, _tree_accounts, _registered_epoch) =
        set_env_with_delegate_and_forester(
            None,
            Some(KeypairActionConfig::all_default()),
            Some(GeneralActionConfig::default()),
            10,
            None,
        )
        .await;

    // let mut env =
    //     E2ETestEnv::<ProgramTestRpcConnection, TestIndexer<ProgramTestRpcConnection>>::new(
    //         rpc,
    //         indexer,
    //         &env_accounts,
    //         KeypairActionConfig::all_default(),
    //         GeneralActionConfig::default(),
    //         10,
    //         None,
    //     )
    //     .await;
    e2e_env.execute_rounds().await;
    println!("stats {:?}", e2e_env.stats);
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
