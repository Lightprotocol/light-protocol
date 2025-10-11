#![cfg(feature = "test-sbf")]

use light_batched_merkle_tree::{
    initialize_address_tree::InitAddressTreeAccountsInstructionData,
    initialize_state_tree::InitStateTreeAccountsInstructionData,
};
use light_program_test::{indexer::TestIndexer, LightProgramTest, ProgramTestConfig};
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
    let mut config = ProgramTestConfig::default_with_batched_trees(true);
    config.v2_state_tree_config = Some(params);
    config.v2_address_tree_config = Some(address_params);
    config.protocol_config = protocol_config;
    config.with_prover = true;
    let rpc = LightProgramTest::new(config).await.unwrap();

    let indexer: TestIndexer = TestIndexer::init_from_acounts(
        &rpc.test_accounts.protocol.forester.insecure_clone(),
        &rpc.test_accounts,
        params.output_queue_batch_size as usize,
    )
    .await;
    let mut config = KeypairActionConfig::test_default();
    config.fee_assert = false;
    let general_config = GeneralActionConfig {
        create_address_mt: None,
        create_state_mt: None,
        rollover: None,
        ..GeneralActionConfig::default()
    };
    let test_accounts = rpc.test_accounts.clone();
    let mut env = E2ETestEnv::<LightProgramTest, TestIndexer>::new(
        rpc,
        indexer,
        &test_accounts,
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
    let mut config = ProgramTestConfig::default_with_batched_trees(true);
    config.v2_state_tree_config = Some(params);
    config.v2_address_tree_config = Some(address_params);
    config.protocol_config = protocol_config;
    config.with_prover = true;
    config.additional_programs = Some(vec![(
        "create_address_test_program",
        CREATE_ADDRESS_TEST_PROGRAM_ID,
    )]);
    let rpc = LightProgramTest::new(config).await.unwrap();
    let test_accounts = rpc.test_accounts.clone();

    let indexer: TestIndexer = TestIndexer::init_from_acounts(
        &test_accounts.protocol.forester.insecure_clone(),
        &test_accounts,
        params.output_queue_batch_size as usize,
    )
    .await;
    let mut config = KeypairActionConfig::test_default();
    config.fee_assert = false;
    let general_config = GeneralActionConfig {
        rollover: None,
        create_address_mt: None,
        create_state_mt: None,
        add_keypair: None,
        add_forester: None,
        ..GeneralActionConfig::default()
    };
    let mut env = E2ETestEnv::<LightProgramTest, TestIndexer>::new(
        rpc,
        indexer,
        &test_accounts,
        config,
        general_config,
        0,
        None,
    )
    .await;
    // remove the two concurrent Merkle trees
    env.indexer.state_merkle_trees.drain(..2);
    env.indexer.address_merkle_trees.remove(0);
    println!(
        "address_merkle_trees {:?}",
        env.indexer.address_merkle_trees.len()
    );
    println!(
        "state_merkle_trees {:?}",
        env.indexer.state_merkle_trees.len()
    );
    for i in 0..100 {
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
    let mut config = ProgramTestConfig::default_with_batched_trees(true);
    config.v2_state_tree_config = Some(params);
    config.v2_address_tree_config = Some(address_params);
    config.protocol_config = protocol_config;
    config.with_prover = true;
    config.additional_programs = Some(vec![(
        "create_address_test_program",
        CREATE_ADDRESS_TEST_PROGRAM_ID,
    )]);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    rpc.indexer = None;
    let test_accounts = rpc.test_accounts.clone();

    let indexer: TestIndexer = TestIndexer::init_from_acounts(
        &test_accounts.protocol.forester.insecure_clone(),
        &test_accounts,
        params.output_queue_batch_size as usize,
    )
    .await;

    let mut env = E2ETestEnv::<LightProgramTest, TestIndexer>::new(
        rpc,
        indexer,
        &test_accounts,
        KeypairActionConfig::all_default_no_fee_assert(),
        GeneralActionConfig::test_with_rollover(),
        10000,
        None,
    )
    .await;
    env.execute_rounds().await;
}
