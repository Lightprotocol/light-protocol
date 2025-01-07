#![cfg(feature = "test-sbf")]

use light_batched_merkle_tree::{
    initialize_address_tree::InitAddressTreeAccountsInstructionData,
    initialize_state_tree::InitStateTreeAccountsInstructionData,
};
use light_program_test::{
    test_env::setup_test_programs_with_accounts_with_protocol_config_and_batched_tree_params,
    test_rpc::ProgramTestRpcConnection,
};
use light_prover_client::gnark::helpers::{ProofType, ProverConfig};
use light_registry::protocol_config::state::ProtocolConfig;
use light_test_utils::{
    e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig},
    indexer::TestIndexer,
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
    let params = InitStateTreeAccountsInstructionData::e2e_test_default();
    let address_params = InitAddressTreeAccountsInstructionData::e2e_test_default();

    let (rpc, env_accounts) =
        setup_test_programs_with_accounts_with_protocol_config_and_batched_tree_params(
            None,
            protocol_config,
            true,
            params,
            address_params,
        )
        .await;

    let indexer: TestIndexer<ProgramTestRpcConnection> = TestIndexer::init_from_env(
        &env_accounts.forester.insecure_clone(),
        &env_accounts,
        Some(ProverConfig {
            run_mode: None,
            circuits: vec![
                ProofType::Inclusion,
                ProofType::NonInclusion,
                ProofType::Combined,
                ProofType::BatchUpdateTest,
                ProofType::BatchAppendWithProofsTest,
            ],
        }),
        None,
    )
    .await;
    let mut config = KeypairActionConfig::test_default();
    config.fee_assert = false;
    let mut general_config = GeneralActionConfig::default();
    general_config.rollover = None;
    general_config.create_address_mt = None;
    general_config.create_state_mt = None;
    let mut env =
        E2ETestEnv::<ProgramTestRpcConnection, TestIndexer<ProgramTestRpcConnection>>::new(
            rpc,
            indexer,
            &env_accounts,
            config,
            general_config,
            10,
            None,
        )
        .await;
    env.execute_rounds().await;
    println!("stats {:?}", env.stats);
}

//  cargo test-sbf -p e2e-test -- --nocapture --ignored --test test_10000_all > output.txt 2>&1
#[ignore = "Not maintained for batched trees."]
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
    let params = InitStateTreeAccountsInstructionData::e2e_test_default();
    let address_params = InitAddressTreeAccountsInstructionData::e2e_test_default();

    let (rpc, env_accounts) =
        setup_test_programs_with_accounts_with_protocol_config_and_batched_tree_params(
            None,
            protocol_config,
            true,
            params,
            address_params,
        )
        .await;

    let indexer: TestIndexer<ProgramTestRpcConnection> = TestIndexer::init_from_env(
        &env_accounts.forester.insecure_clone(),
        &env_accounts,
        Some(ProverConfig {
            run_mode: None,
            circuits: vec![
                ProofType::Inclusion,
                ProofType::NonInclusion,
                ProofType::Combined,
                ProofType::BatchUpdateTest,
                ProofType::BatchAppendWithProofsTest,
            ],
        }),
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
