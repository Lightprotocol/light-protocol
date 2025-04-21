#![cfg(feature = "test-sbf")]

use light_batched_merkle_tree::{
    initialize_address_tree::InitAddressTreeAccountsInstructionData,
    initialize_state_tree::InitStateTreeAccountsInstructionData,
};
use light_program_test::{
    indexer::TestIndexer,
    test_env::setup_test_programs_with_accounts_with_protocol_config_and_batched_tree_params,
    test_rpc::ProgramTestRpcConnection,
};
use light_prover_client::gnark::helpers::{ProofType, ProverConfig};
use light_registry::protocol_config::state::ProtocolConfig;
use light_test_utils::{
    e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig},
    CREATE_ADDRESS_TEST_PROGRAM_ID,
};
use serial_test::serial;

#[serial]
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

#[serial]
#[tokio::test]
async fn test_batched_only() {
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
            Some(vec![(
                "create_address_test_program",
                CREATE_ADDRESS_TEST_PROGRAM_ID,
            )]),
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
    let mut config = KeypairActionConfig::test_default();
    config.fee_assert = false;
    let mut general_config = GeneralActionConfig::default();
    general_config.rollover = None;
    general_config.create_address_mt = None;
    general_config.create_state_mt = None;
    general_config.add_keypair = None;
    general_config.rollover = None;
    general_config.add_forester = None;
    let mut env =
        E2ETestEnv::<ProgramTestRpcConnection, TestIndexer<ProgramTestRpcConnection>>::new(
            rpc,
            indexer,
            &env_accounts,
            config,
            general_config,
            0,
            None,
        )
        .await;
    // remove concurrent Merkle trees
    env.indexer.state_merkle_trees.remove(0);
    env.indexer.address_merkle_trees.remove(0);
    println!(
        "address_merkle_trees {:?}",
        env.indexer.address_merkle_trees.len()
    );
    println!(
        "state_merkle_trees {:?}",
        env.indexer.state_merkle_trees.len()
    );
    for i in 0..10000 {
        println!("\n\nround {:?}\n\n", i);
        env.invoke_cpi_test(0).await.unwrap();
        env.activate_general_actions().await;
    }
    println!("stats {:?}", env.stats);
}

//  cargo test-sbf -p e2e-test -- --nocapture --ignored --test test_10000_all > output.txt 2>&1 && tail -f output.txt
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
            Some(8464865003173904667),
        )
        .await;
    env.execute_rounds().await;
}
