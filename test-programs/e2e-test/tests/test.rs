#![cfg(feature = "test-sbf")]

use light_registry::protocol_config::state::ProtocolConfig;
use light_test_utils::e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig};
use light_test_utils::indexer::TestIndexer;
use light_test_utils::rpc::ProgramTestRpcConnection;
use light_test_utils::test_env::{
    set_env_with_delegate_and_forester, set_env_with_delegate_and_forester_local,
    setup_test_programs_with_accounts_with_protocol_config,
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
    let (rpc, indexer, _delegate_keypair, env_accounts, _tree_accounts, registered_epoch) =
        set_env_with_delegate_and_forester_local(None).await;

    let mut e2e_env =
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
    // let _forester = Forester {
    //     registration: registered_epoch.clone(),
    //     active: registered_epoch.clone(),
    //     ..Default::default()
    // };
    // // Forester epoch account is assumed to exist (is inited with test program deployment)
    // let forester = TestForester {
    //     keypair: env_accounts.forester.insecure_clone(),
    //     forester: _forester.clone(),
    //     is_registered: Some(0),
    // };
    // e2e_env.foresters.push(forester);
    // e2e_env.epoch_config = _forester;
    // e2e_env.epoch = registered_epoch.epoch;
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
