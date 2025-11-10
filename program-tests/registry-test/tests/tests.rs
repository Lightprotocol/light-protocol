use std::collections::HashSet;

use account_compression::{
    errors::AccountCompressionErrorCode, AddressMerkleTreeConfig, AddressQueueConfig,
    MigrateLeafParams, NullifierQueueConfig, StateMerkleTreeAccount, StateMerkleTreeConfig,
};
use anchor_lang::{AnchorSerialize, InstructionData, ToAccountMetas};
use forester_utils::{
    account_zero_copy::get_concurrent_merkle_tree, forester_epoch::get_epoch_phases,
    utils::airdrop_lamports,
};
use light_account_checks::error::AccountError;
use light_batched_merkle_tree::{
    initialize_address_tree::InitAddressTreeAccountsInstructionData,
    initialize_state_tree::test_utils::{
        assert_address_mt_zero_copy_initialized, InitStateTreeAccountsInstructionData,
    },
    merkle_tree::BatchedMerkleTreeAccount,
    merkle_tree_metadata::{BatchedMerkleTreeMetadata, CreateTreeParams},
    queue::BatchedQueueAccount,
};
use light_client::{indexer::Indexer, rpc::LightClientConfig};
use light_compressed_account::TreeType;
use light_hasher::Poseidon;
use light_program_test::{
    accounts::{
        address_tree::create_address_merkle_tree_and_queue_account,
        address_tree_v2::create_batch_address_merkle_tree,
        initialize::initialize_new_group,
        register_program::{
            deregister_program_with_registry_program, register_program_with_registry_program,
        },
        state_tree::create_state_merkle_tree_and_queue_account,
        test_accounts::{TestAccounts, NOOP_PROGRAM_ID},
        test_keypairs::{GROUP_PDA_SEED_TEST_KEYPAIR, OLD_REGISTRY_ID_TEST_KEYPAIR},
    },
    indexer::{TestIndexer, TestIndexerExtensions},
    program_test::{LightProgramTest, TestRpc},
    utils::{assert::assert_rpc_error, setup_light_programs::setup_light_programs},
    ProgramTestConfig,
};
use light_registry::{
    account_compression_cpi::sdk::{
        create_batch_append_instruction, create_batch_nullify_instruction,
        create_batch_update_address_tree_instruction, create_migrate_state_instruction,
        create_nullify_instruction, create_update_address_merkle_tree_instruction,
        CreateMigrateStateInstructionInputs, CreateNullifyInstructionInputs,
        UpdateAddressMerkleTreeInstructionInputs,
    },
    errors::RegistryError,
    protocol_config::state::{ProtocolConfig, ProtocolConfigPda},
    sdk::{
        create_finalize_registration_instruction, create_report_work_instruction,
        create_update_forester_pda_weight_instruction,
    },
    utils::{
        get_cpi_authority_pda, get_forester_epoch_pda_from_authority, get_forester_pda,
        get_protocol_config_pda_address,
    },
    ForesterConfig, ForesterEpochPda, ForesterPda,
};
use light_test_utils::{
    assert_epoch::{
        assert_epoch_pda, assert_finalized_epoch_registration, assert_registered_forester_pda,
        assert_report_work, fetch_epoch_and_forester_pdas,
    },
    create_address_merkle_tree_and_queue_account_with_assert,
    create_address_test_program_sdk::perform_create_pda_with_event_rnd,
    create_rollover_address_merkle_tree_instructions,
    create_rollover_state_merkle_tree_instructions,
    e2e_test_env::init_program_test_env,
    register_test_forester,
    setup_accounts::setup_accounts,
    test_batch_forester::{
        assert_perform_state_mt_roll_over, create_append_batch_ix_data,
        create_batch_update_address_tree_instruction_data_with_proof, perform_batch_append,
        perform_batch_nullify, perform_rollover_batch_address_merkle_tree,
        perform_rollover_batch_state_merkle_tree,
    },
    test_forester::{empty_address_queue_test, nullify_compressed_accounts},
    test_keypairs::from_target_folder,
    update_test_forester, Epoch, LightClient, Rpc, RpcError, RpcUrl, TreeAccounts,
    CREATE_ADDRESS_TEST_PROGRAM_ID,
};
use serial_test::serial;
use solana_sdk::{
    account::WritableAccount,
    instruction::Instruction,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signature},
    signer::Signer,
};

#[test]
fn test_protocol_config_active_phase_continuity() {
    let devnet_config = ProtocolConfig {
        genesis_slot: 0,
        min_weight: 1,
        slot_length: 10,
        registration_phase_length: 100,
        active_phase_length: 1000,
        report_work_phase_length: 100,
        network_fee: 5000,
        cpi_context_size: 20488,
        finalize_counter_limit: 100,
        place_holder: Pubkey::default(),
        address_network_fee: 10000,
        place_holder_b: 0,
        place_holder_c: 0,
        place_holder_d: 0,
        place_holder_e: 0,
        place_holder_f: 0,
    };

    let mainnet_config = ProtocolConfig {
        genesis_slot: 286142505,
        min_weight: 1,
        slot_length: 50,
        registration_phase_length: 216000,
        active_phase_length: 432000,
        report_work_phase_length: 216000,
        network_fee: 5000,
        cpi_context_size: 20488,
        finalize_counter_limit: 100,
        place_holder: Pubkey::default(),
        address_network_fee: 10000,
        place_holder_b: 0,
        place_holder_c: 0,
        place_holder_d: 0,
        place_holder_e: 0,
        place_holder_f: 0,
    };

    let configs = vec![devnet_config, mainnet_config];
    for config in configs {
        test_protocol_config_active_phase_continuity_for_config(config);
    }
}

// Test that each slot is active in exactly one epoch
fn test_protocol_config_active_phase_continuity_for_config(config: ProtocolConfig) {
    // Test for 10 epochs
    let epochs = 10;

    let total_slots_to_test = config.active_phase_length * epochs;

    for slot in config.genesis_slot..(config.genesis_slot + total_slots_to_test) {
        if slot < config.genesis_slot + config.registration_phase_length {
            // assert that is registration phase
            assert_eq!(config.get_latest_register_epoch(slot).unwrap(), 0);
            continue;
        }
        let mut active_epochs = HashSet::new();
        for offset in -1..1 {
            let epoch = config.get_current_epoch(slot) as i64 + offset;
            if epoch < 0 {
                continue;
            }

            let phases = get_epoch_phases(&config, epoch as u64);

            if slot >= phases.active.start && slot <= phases.active.end {
                active_epochs.insert(epoch);
            }
        }

        assert_eq!(
            active_epochs.len(),
            1,
            "Slot {} should be active in exactly one epoch, but was active in {} epochs. Protocol config: {:?}",
            slot,
            active_epochs.len(),
            config
        );
    }
}

#[tokio::test]
async fn test_initialize_protocol_config() {
    let mut context = setup_light_programs(None).unwrap();
    let payer = Keypair::new();
    context
        .airdrop(&payer.pubkey(), 100_000_000_000_000)
        .unwrap();
    let mut rpc = LightProgramTest {
        context,
        indexer: None,
        test_accounts: TestAccounts::get_program_test_test_accounts(),
        payer,
        config: ProgramTestConfig::default(),
        transaction_counter: 0,
        pre_context: None,
    };

    let payer = rpc.get_payer().insecure_clone();
    let program_account_keypair =
        Keypair::try_from(OLD_REGISTRY_ID_TEST_KEYPAIR.as_slice()).unwrap();
    let protocol_config = ProtocolConfig::default();
    let (protocol_config_pda, bump) = get_protocol_config_pda_address();
    let ix_data = light_registry::instruction::InitializeProtocolConfig {
        protocol_config,
        bump,
    };
    // // init with invalid authority
    // {
    //     let accounts = light_registry::accounts::InitializeProtocolConfig {
    //         protocol_config_pda,
    //         authority: payer.pubkey(),
    //         fee_payer: payer.pubkey(),
    //         system_program: solana_sdk::system_program::id(),
    //         self_program: light_registry::ID,
    //     };
    //     let ix = Instruction {
    //         program_id: light_registry::ID,
    //         accounts: accounts.to_account_metas(Some(true)),
    //         data: ix_data.data(),
    //     };
    //     let result = rpc
    //         .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
    //         .await;
    //     assert_rpc_error(
    //         result,
    //         0,
    //         anchor_lang::error::ErrorCode::ConstraintRaw as u32,
    //     )
    //     .unwrap();
    // }
    // init with valid authority
    {
        let accounts = light_registry::accounts::InitializeProtocolConfig {
            protocol_config_pda,
            authority: program_account_keypair.pubkey(),
            fee_payer: payer.pubkey(),
            system_program: solana_sdk::system_program::id(),
            self_program: light_registry::ID,
        };
        let ix = Instruction {
            program_id: light_registry::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: ix_data.data(),
        };
        rpc.create_and_send_transaction(
            &[ix],
            &payer.pubkey(),
            &[&payer, &program_account_keypair],
        )
        .await
        .unwrap();
        let protocol_config_pda: ProtocolConfigPda = rpc
            .get_anchor_account::<ProtocolConfigPda>(&protocol_config_pda)
            .await
            .unwrap()
            .unwrap();
        println!("protocol_config_pda: {:?}", protocol_config_pda);
        assert_eq!(
            protocol_config_pda.authority,
            program_account_keypair.pubkey()
        );
        assert_eq!(protocol_config_pda.config, protocol_config);
        assert_eq!(protocol_config_pda.bump, bump);
    }

    // Test: update protocol config

    let updated_keypair = Keypair::new();
    rpc.airdrop_lamports(&updated_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // fail to update protocol config with invalid authority
    {
        let instruction = light_registry::instruction::UpdateProtocolConfig {
            protocol_config: None,
        };
        let accounts = light_registry::accounts::UpdateProtocolConfig {
            protocol_config_pda,
            authority: payer.pubkey(),
            new_authority: Some(updated_keypair.pubkey()),
            fee_payer: payer.pubkey(),
        };
        let ix = Instruction {
            program_id: light_registry::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &updated_keypair])
            .await;
        assert_rpc_error(
            result,
            0,
            anchor_lang::error::ErrorCode::ConstraintHasOne as u32,
        )
        .unwrap();
    }
    {
        let updated_protocol_config = ProtocolConfig {
            registration_phase_length: 123,
            report_work_phase_length: 123,
            ..Default::default()
        };

        let instruction = light_registry::instruction::UpdateProtocolConfig {
            protocol_config: Some(updated_protocol_config),
        };
        let accounts = light_registry::accounts::UpdateProtocolConfig {
            protocol_config_pda,
            authority: program_account_keypair.pubkey(),
            new_authority: Some(updated_keypair.pubkey()),
            fee_payer: payer.pubkey(),
        };
        let ix = Instruction {
            program_id: light_registry::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction.data(),
        };
        rpc.create_and_send_transaction(
            &[ix],
            &payer.pubkey(),
            &[&payer, &updated_keypair, &program_account_keypair],
        )
        .await
        .unwrap();
        let governance_authority = rpc
            .get_anchor_account::<ProtocolConfigPda>(&protocol_config_pda)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(governance_authority.authority, updated_keypair.pubkey());
        assert_eq!(governance_authority.config, updated_protocol_config);
    }
    let cpi_authority_pda = get_cpi_authority_pda();

    let group_seed_keypair = Keypair::try_from(GROUP_PDA_SEED_TEST_KEYPAIR.as_slice()).unwrap();
    let group_pda =
        initialize_new_group(&group_seed_keypair, &payer, &mut rpc, cpi_authority_pda.0)
            .await
            .unwrap();

    let random_program_keypair = Keypair::new();
    // register program with invalid authority
    {
        let result = register_program_with_registry_program(
            &mut rpc,
            &payer,
            &group_pda,
            &random_program_keypair,
        )
        .await;
        let expected_error_code = anchor_lang::error::ErrorCode::ConstraintHasOne as u32;
        assert_rpc_error(result, 1, expected_error_code).unwrap();
    }
    // deregister program functional and with invalid signer
    {
        let random_program_keypair = Keypair::new();
        register_program_with_registry_program(
            &mut rpc,
            &updated_keypair,
            &group_pda,
            &random_program_keypair,
        )
        .await
        .unwrap();
        let result = deregister_program_with_registry_program(
            &mut rpc,
            &payer,
            &group_pda,
            &random_program_keypair,
        )
        .await;
        let expected_error_code = anchor_lang::error::ErrorCode::ConstraintHasOne as u32;
        assert_rpc_error(result, 1, expected_error_code).unwrap();
        deregister_program_with_registry_program(
            &mut rpc,
            &updated_keypair,
            &group_pda,
            &random_program_keypair,
        )
        .await
        .unwrap();
    }
    // initialize a Merkle tree with network fee = 0
    {
        let merkle_tree_keypair = Keypair::new();
        let nullifier_queue_keypair = Keypair::new();
        let cpi_context_keypair = Keypair::new();
        create_state_merkle_tree_and_queue_account(
            &payer,
            true,
            &mut rpc,
            &merkle_tree_keypair,
            &nullifier_queue_keypair,
            Some(&cpi_context_keypair),
            None,
            Some(Pubkey::new_unique()),
            1,
            &StateMerkleTreeConfig {
                network_fee: None,
                ..Default::default()
            },
            &NullifierQueueConfig::default(),
        )
        .await
        .unwrap();
    }
    // initialize a Merkle tree with network fee = 0
    {
        let merkle_tree_keypair = Keypair::new();
        let nullifier_queue_keypair = Keypair::new();
        let cpi_context_keypair = Keypair::new();
        let result = create_state_merkle_tree_and_queue_account(
            &payer,
            true,
            &mut rpc,
            &merkle_tree_keypair,
            &nullifier_queue_keypair,
            Some(&cpi_context_keypair),
            None,
            None,
            1,
            &StateMerkleTreeConfig {
                network_fee: None,
                ..Default::default()
            },
            &NullifierQueueConfig::default(),
        )
        .await;
        assert_rpc_error(result, 3, RegistryError::ForesterUndefined.into()).unwrap();
    }
    // initialize a Merkle tree with network fee = 5000 (default)
    {
        let merkle_tree_keypair = Keypair::new();
        let nullifier_queue_keypair = Keypair::new();
        let cpi_context_keypair = Keypair::new();
        create_state_merkle_tree_and_queue_account(
            &payer,
            true,
            &mut rpc,
            &merkle_tree_keypair,
            &nullifier_queue_keypair,
            Some(&cpi_context_keypair),
            None,
            None,
            1,
            &StateMerkleTreeConfig {
                network_fee: Some(5000),
                ..Default::default()
            },
            &NullifierQueueConfig::default(),
        )
        .await
        .unwrap();
    }
    // FAIL: initialize a Merkle tree with network fee != 0 || 5000
    {
        let merkle_tree_keypair = Keypair::new();
        let nullifier_queue_keypair = Keypair::new();
        let cpi_context_keypair = Keypair::new();
        let result = create_state_merkle_tree_and_queue_account(
            &payer,
            true,
            &mut rpc,
            &merkle_tree_keypair,
            &nullifier_queue_keypair,
            Some(&cpi_context_keypair),
            None,
            None,
            1,
            &StateMerkleTreeConfig {
                network_fee: Some(5001),
                ..Default::default()
            },
            &NullifierQueueConfig::default(),
        )
        .await;
        let expected_error_code = RegistryError::InvalidNetworkFee as u32 + 6000;
        assert_rpc_error(result, 3, expected_error_code).unwrap();
    }
    // initialize a Merkle tree with network fee = 0
    {
        let merkle_tree_keypair = Keypair::new();
        let queue_keypair = Keypair::new();
        create_address_merkle_tree_and_queue_account_with_assert(
            &payer,
            true,
            &mut rpc,
            &merkle_tree_keypair,
            &queue_keypair,
            Some(Pubkey::new_unique()),
            Some(Pubkey::new_unique()),
            &AddressMerkleTreeConfig {
                network_fee: None,
                ..Default::default()
            },
            &AddressQueueConfig {
                network_fee: None,
                ..Default::default()
            },
            0,
        )
        .await
        .unwrap();
    }
    // Deprecated should fail
    // initialize a Merkle tree without a forester
    {
        let merkle_tree_keypair = Keypair::new();
        let queue_keypair = Keypair::new();
        let result = create_address_merkle_tree_and_queue_account_with_assert(
            &payer,
            true,
            &mut rpc,
            &merkle_tree_keypair,
            &queue_keypair,
            Some(Pubkey::new_unique()),
            None,
            &AddressMerkleTreeConfig {
                network_fee: None,
                ..Default::default()
            },
            &AddressQueueConfig::default(),
            0,
        )
        .await;
        assert_rpc_error(result, 3, RegistryError::ForesterUndefined.into()).unwrap();
    }
    // initialize a Merkle tree without a Program owner
    {
        let merkle_tree_keypair = Keypair::new();
        let queue_keypair = Keypair::new();
        let result = create_address_merkle_tree_and_queue_account_with_assert(
            &payer,
            true,
            &mut rpc,
            &merkle_tree_keypair,
            &queue_keypair,
            None,
            Some(Pubkey::new_unique()),
            &AddressMerkleTreeConfig {
                network_fee: None,
                ..Default::default()
            },
            &AddressQueueConfig::default(),
            0,
        )
        .await;
        assert_rpc_error(result, 3, RegistryError::ProgramOwnerUndefined.into()).unwrap();
    }
    // FAIL: initialize a Merkle tree with network fee != 0
    {
        let merkle_tree_keypair = Keypair::new();
        let queue_keypair = Keypair::new();
        let result = create_address_merkle_tree_and_queue_account(
            &payer,
            true,
            &mut rpc,
            &merkle_tree_keypair,
            &queue_keypair,
            Some(Pubkey::new_unique()),
            Some(Pubkey::new_unique()),
            &AddressMerkleTreeConfig {
                network_fee: Some(10000),
                ..Default::default()
            },
            &AddressQueueConfig::default(),
            0,
        )
        .await;
        assert_rpc_error(result, 3, RegistryError::InvalidNetworkFee.into()).unwrap();
    }
}

#[serial]
#[tokio::test]
async fn test_custom_forester() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::default_with_batched_trees(true))
        .await
        .unwrap();

    rpc.indexer = None;

    let env = rpc.test_accounts.clone();
    let payer = rpc.get_payer().insecure_clone();
    {
        let unregistered_forester_keypair = Keypair::new();
        rpc.airdrop_lamports(&unregistered_forester_keypair.pubkey(), 1_000_000_000)
            .await
            .unwrap();
        let merkle_tree_keypair = Keypair::new();
        let nullifier_queue_keypair = Keypair::new();
        let cpi_context_keypair = Keypair::new();
        // create work 1 item in address and nullifier queue each
        let (mut state_merkle_tree_bundle, _, mut rpc) = {
            let mut e2e_env = init_program_test_env(rpc, &env, 50).await;
            e2e_env.indexer.state_merkle_trees.clear();
            // add state merkle tree to the indexer
            e2e_env
                .indexer
                .add_state_merkle_tree(
                    &mut e2e_env.rpc,
                    &merkle_tree_keypair,
                    &nullifier_queue_keypair,
                    &cpi_context_keypair,
                    None,
                    Some(unregistered_forester_keypair.pubkey()),
                    TreeType::StateV1,
                )
                .await;

            // e2e_env.create_address(None).await;
            e2e_env
                .compress_sol_deterministic(&unregistered_forester_keypair, 1_000_000, None)
                .await;
            e2e_env
                .transfer_sol_deterministic(
                    &unregistered_forester_keypair,
                    &Pubkey::new_unique(),
                    None,
                )
                .await
                .unwrap();

            (
                e2e_env.indexer.state_merkle_trees[0].clone(),
                e2e_env.indexer.address_merkle_trees[0].clone(),
                e2e_env.rpc,
            )
        };
        {
            let result = nullify_compressed_accounts(
                &mut rpc,
                &payer,
                &mut state_merkle_tree_bundle,
                0,
                true,
            )
            .await;
            assert_rpc_error(result, 0, RegistryError::InvalidSigner.into()).unwrap();
        }
        // nullify with tree forester
        nullify_compressed_accounts(
            &mut rpc,
            &unregistered_forester_keypair,
            &mut state_merkle_tree_bundle,
            0,
            true,
        )
        .await
        .unwrap();
    }
}

#[serial]
#[tokio::test]
async fn test_custom_forester_batched() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::default_test_forester(true))
        .await
        .unwrap();

    rpc.indexer = None;
    let env = rpc.test_accounts.clone();
    let tree_params = ProgramTestConfig::default_with_batched_trees(true)
        .v2_state_tree_config
        .unwrap();

    {
        let mut instruction_data = None;
        let unregistered_forester_keypair = Keypair::new();
        rpc.airdrop_lamports(&unregistered_forester_keypair.pubkey(), 1_000_000_000)
            .await
            .unwrap();
        let merkle_tree_keypair = Keypair::new();
        let nullifier_queue_keypair = Keypair::new();
        let cpi_context_keypair = Keypair::new();
        // create work 1 item in address and nullifier queue each
        let (mut state_merkle_tree_bundle, _, mut rpc) = {
            let mut e2e_env = init_program_test_env(rpc, &env, 50).await;
            e2e_env.indexer.state_merkle_trees.clear();

            // add state merkle tree to the indexer
            e2e_env
                .indexer
                .add_state_merkle_tree(
                    &mut e2e_env.rpc,
                    &merkle_tree_keypair,
                    &nullifier_queue_keypair,
                    &cpi_context_keypair,
                    None,
                    None,
                    TreeType::StateV2,
                )
                .await;
            let state_merkle_tree_pubkey =
                e2e_env.indexer.state_merkle_trees[0].accounts.merkle_tree;

            let mut merkle_tree_account = e2e_env
                .rpc
                .get_account(state_merkle_tree_pubkey)
                .await
                .unwrap()
                .unwrap();
            let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
                &mut merkle_tree_account.data,
                &state_merkle_tree_pubkey.into(),
            )
            .unwrap();
            // fill two output and one input batch
            for i in 0..merkle_tree.get_metadata().queue_batches.batch_size {
                println!("\ntx {}", i);

                e2e_env
                    .compress_sol_deterministic(&unregistered_forester_keypair, 1_000_000, None)
                    .await;
                e2e_env
                    .transfer_sol_deterministic(
                        &unregistered_forester_keypair,
                        &Pubkey::new_unique(),
                        None,
                    )
                    .await
                    .unwrap();
                if i == merkle_tree.get_metadata().queue_batches.batch_size / 2 {
                    instruction_data = Some(
                        create_append_batch_ix_data(
                            &mut e2e_env.rpc,
                            &mut e2e_env.indexer.state_merkle_trees[0],
                        )
                        .await,
                    );
                }
            }
            (
                e2e_env.indexer.state_merkle_trees[0].clone(),
                e2e_env.indexer.address_merkle_trees[0].clone(),
                e2e_env.rpc,
            )
        };
        let num_output_zkp_batches =
            tree_params.input_queue_batch_size / tree_params.output_queue_zkp_batch_size;
        for i in 0..num_output_zkp_batches {
            // Simulate concurrency since instruction data has been created before
            let instruction_data = if i == 0 { instruction_data } else { None };
            perform_batch_append(
                &mut rpc,
                &mut state_merkle_tree_bundle,
                &env.protocol.forester,
                0,
                false,
                instruction_data,
            )
            .await
            .unwrap();
            // We only spent half of the output queue
            if i < num_output_zkp_batches / 2 {
                perform_batch_nullify(
                    &mut rpc,
                    &mut state_merkle_tree_bundle,
                    &env.protocol.forester,
                    0,
                    false,
                    None,
                )
                .await
                .unwrap();
            }
        }
    }
}

/// Test:
/// 1. SUCCESS: Register a forester
/// 2. SUCCESS: Update forester authority
/// 3. SUCCESS: Register forester for epoch
#[serial]
#[tokio::test]
async fn test_register_and_update_forester_pda() {
    let config = ProgramTestConfig {
        protocol_config: ProtocolConfig::default(),
        with_prover: false,
        with_forester: false,
        ..Default::default()
    };

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    rpc.indexer = None;
    let env = rpc.test_accounts.clone();
    let forester_keypair = Keypair::new();
    rpc.airdrop_lamports(&forester_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let config = ForesterConfig { fee: 1 };
    // 1. SUCCESS: Register a forester
    register_test_forester(
        &mut rpc,
        &env.protocol.governance_authority,
        &forester_keypair.pubkey(),
        config,
    )
    .await
    .unwrap();

    // 2. SUCCESS: Update forester authority
    let new_forester_keypair = Keypair::new();
    rpc.airdrop_lamports(&new_forester_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let config = ForesterConfig { fee: 2 };

    update_test_forester(
        &mut rpc,
        &forester_keypair,
        &forester_keypair.pubkey(),
        Some(&new_forester_keypair),
        config,
    )
    .await
    .unwrap();
    // change the forester authority back
    update_test_forester(
        &mut rpc,
        &new_forester_keypair,
        &forester_keypair.pubkey(),
        Some(&forester_keypair),
        config,
    )
    .await
    .unwrap();
    let protocol_config = rpc
        .get_anchor_account::<ProtocolConfigPda>(&env.protocol.governance_authority_pda)
        .await
        .unwrap()
        .unwrap()
        .config;

    // SUCCESS: update forester weight
    {
        let ix = create_update_forester_pda_weight_instruction(
            &forester_keypair.pubkey(),
            &env.protocol.governance_authority.pubkey(),
            11,
        );
        rpc.create_and_send_transaction(
            &[ix],
            &env.protocol.governance_authority.pubkey(),
            &[&env.protocol.governance_authority],
        )
        .await
        .unwrap();
        let forester_pda: ForesterPda = rpc
            .get_anchor_account::<ForesterPda>(&get_forester_pda(&forester_keypair.pubkey()).0)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(forester_pda.active_weight, 11);
        // change it back because other asserts expect weight 1
        let ix = create_update_forester_pda_weight_instruction(
            &forester_keypair.pubkey(),
            &env.protocol.governance_authority.pubkey(),
            1,
        );
        rpc.create_and_send_transaction(
            &[ix],
            &env.protocol.governance_authority.pubkey(),
            &[&env.protocol.governance_authority],
        )
        .await
        .unwrap();
    }

    // 3. SUCCESS: register forester for epoch
    let tree_accounts = vec![
        TreeAccounts {
            tree_type: TreeType::StateV1,
            merkle_tree: env.v1_state_trees[0].merkle_tree,
            queue: env.v1_state_trees[0].nullifier_queue,
            is_rolledover: false,
        },
        TreeAccounts {
            tree_type: TreeType::AddressV1,
            merkle_tree: env.v1_address_trees[0].merkle_tree,
            queue: env.v1_address_trees[0].queue,
            is_rolledover: false,
        },
    ];

    let registered_epoch = Epoch::register(
        &mut rpc,
        &protocol_config,
        &forester_keypair,
        &forester_keypair.pubkey(),
    )
    .await
    .unwrap();
    assert!(registered_epoch.is_some());
    let mut registered_epoch = registered_epoch.unwrap();
    let forester_epoch_pda = rpc
        .get_anchor_account::<ForesterEpochPda>(&registered_epoch.forester_epoch_pda)
        .await
        .unwrap()
        .unwrap();
    assert!(forester_epoch_pda.total_epoch_weight.is_none());
    assert_eq!(forester_epoch_pda.epoch, 0);
    let epoch = 0;
    let expected_stake = 1;
    assert_epoch_pda(&mut rpc, epoch, expected_stake).await;
    assert_registered_forester_pda(
        &mut rpc,
        &registered_epoch.forester_epoch_pda,
        &forester_keypair.pubkey(),
        epoch,
    )
    .await;

    // advance epoch to active phase
    rpc.warp_to_slot(registered_epoch.phases.active.start)
        .unwrap();
    // finalize registration
    {
        registered_epoch
            .fetch_account_and_add_trees_with_schedule(&mut rpc, &tree_accounts)
            .await
            .unwrap();
        let ix = create_finalize_registration_instruction(
            &forester_keypair.pubkey(),
            &forester_keypair.pubkey(),
            registered_epoch.epoch,
        );
        rpc.create_and_send_transaction(&[ix], &forester_keypair.pubkey(), &[&forester_keypair])
            .await
            .unwrap();
        assert_finalized_epoch_registration(
            &mut rpc,
            &registered_epoch.forester_epoch_pda,
            &registered_epoch.epoch_pda,
        )
        .await;
    }

    // create work 1 item in address and nullifier queue each
    let (mut state_merkle_tree_bundle, mut address_merkle_tree, mut rpc) = {
        let mut e2e_env = init_program_test_env(rpc, &env, 50).await;
        // remove batched Merkle tree, fee assert makes this test flaky otherwise
        e2e_env.indexer.state_merkle_trees.truncate(1);
        e2e_env.create_address(None, None).await;
        e2e_env
            .compress_sol_deterministic(&forester_keypair, 1_000_000, None)
            .await;
        e2e_env
            .transfer_sol_deterministic(&forester_keypair, &Pubkey::new_unique(), None)
            .await
            .unwrap();

        (
            e2e_env.indexer.state_merkle_trees[0].clone(),
            e2e_env.indexer.address_merkle_trees[0].clone(),
            e2e_env.rpc,
        )
    };
    // perform 1 work
    nullify_compressed_accounts(
        &mut rpc,
        &forester_keypair,
        &mut state_merkle_tree_bundle,
        epoch,
        false,
    )
    .await
    .unwrap();
    empty_address_queue_test(
        &forester_keypair,
        &mut rpc,
        &mut address_merkle_tree,
        false,
        epoch,
        false,
    )
    .await
    .unwrap();

    // advance epoch to report work and next registration phase
    rpc.warp_to_slot(
        registered_epoch.phases.report_work.start - protocol_config.registration_phase_length,
    )
    .unwrap();
    // register for next epoch
    let next_registered_epoch = Epoch::register(
        &mut rpc,
        &protocol_config,
        &forester_keypair,
        &forester_keypair.pubkey(),
    )
    .await
    .unwrap();
    assert!(next_registered_epoch.is_some());
    let next_registered_epoch = next_registered_epoch.unwrap();
    assert_eq!(next_registered_epoch.epoch, 1);
    assert_epoch_pda(&mut rpc, next_registered_epoch.epoch, expected_stake).await;
    assert_registered_forester_pda(
        &mut rpc,
        &next_registered_epoch.forester_epoch_pda,
        &forester_keypair.pubkey(),
        next_registered_epoch.epoch,
    )
    .await;
    rpc.warp_to_slot(registered_epoch.phases.report_work.start)
        .unwrap();
    // report work
    {
        let (pre_forester_epoch_pda, pre_epoch_pda) = fetch_epoch_and_forester_pdas(
            &mut rpc,
            &registered_epoch.forester_epoch_pda,
            &registered_epoch.epoch_pda,
        )
        .await;
        let ix = create_report_work_instruction(
            &forester_keypair.pubkey(),
            &forester_keypair.pubkey(),
            registered_epoch.epoch,
        );
        rpc.create_and_send_transaction(&[ix], &forester_keypair.pubkey(), &[&forester_keypair])
            .await
            .unwrap();
        assert_report_work(
            &mut rpc,
            &registered_epoch.forester_epoch_pda,
            &registered_epoch.epoch_pda,
            pre_forester_epoch_pda,
            pre_epoch_pda,
        )
        .await;
    }
}

// TODO: fix numbering
/// Test:
/// 1. FAIL: Register a forester with invalid authority
/// 2. FAIL: Update forester pda authority with invalid authority
/// 2. FAIL: Update forester epoch pda authority with invalid authority
/// 3. FAIL: Nullify with invalid authority
/// 4. FAIL: Update address tree with invalid authority
/// 5. FAIL: Rollover address tree with invalid authority
/// 6. FAIL: Rollover state tree with invalid authority
#[serial]
#[tokio::test]
async fn failing_test_forester() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::default_with_batched_trees(true))
        .await
        .unwrap();

    rpc.indexer = None;
    let env = rpc.test_accounts.clone();
    let payer = rpc.get_payer().insecure_clone();
    assert_ne!(payer.pubkey(), env.protocol.forester.pubkey());
    // 1. FAIL: Register a forester pda with invalid authority
    {
        let result = register_test_forester(
            &mut rpc,
            &payer,
            &Keypair::new().pubkey(),
            ForesterConfig::default(),
        )
        .await;
        let expected_error_code = anchor_lang::error::ErrorCode::ConstraintHasOne as u32;
        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 2. FAIL: Update forester pda with invalid authority
    {
        let forester_pda = get_forester_pda(&env.protocol.forester.pubkey()).0;
        let instruction_data = light_registry::instruction::UpdateForesterPda { config: None };
        let accounts = light_registry::accounts::UpdateForesterPda {
            forester_pda,
            authority: payer.pubkey(),
            new_authority: Some(payer.pubkey()),
        };
        let ix = Instruction {
            program_id: light_registry::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
            .await;
        let expected_error_code = anchor_lang::error::ErrorCode::ConstraintHasOne as u32;
        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 3. FAIL: Update forester pda weight with invalid authority
    {
        let ix = light_registry::instruction::UpdateForesterPdaWeight { new_weight: 11 };
        let accounts = light_registry::accounts::UpdateForesterPdaWeight {
            forester_pda: get_forester_pda(&env.protocol.forester.pubkey()).0,
            authority: payer.pubkey(),
            protocol_config_pda: env.protocol.governance_authority_pda,
        };
        let ix = Instruction {
            program_id: light_registry::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: ix.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
            .await;
        let expected_error_code = anchor_lang::error::ErrorCode::ConstraintHasOne as u32;
        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 4. FAIL: Nullify with invalid authority
    {
        let expected_error_code = RegistryError::InvalidForester as u32 + 6000;
        let inputs = CreateNullifyInstructionInputs {
            authority: payer.pubkey(),
            nullifier_queue: env.v1_state_trees[0].nullifier_queue,
            merkle_tree: env.v1_state_trees[0].merkle_tree,
            change_log_indices: vec![1],
            leaves_queue_indices: vec![1u16],
            indices: vec![0u64],
            proofs: vec![vec![[0u8; 32]; 26]],
            derivation: payer.pubkey(),
            is_metadata_forester: false,
        };
        let mut ix = create_nullify_instruction(inputs, 0);
        // Swap the derived forester pda with an initialized but invalid one.
        ix.accounts[0].pubkey =
            get_forester_epoch_pda_from_authority(&env.protocol.forester.pubkey(), 0).0;
        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
            .await;
        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 4 FAIL: update address Merkle tree failed
    {
        let authority = rpc.get_payer().insecure_clone();
        let mut instruction = create_update_address_merkle_tree_instruction(
            UpdateAddressMerkleTreeInstructionInputs {
                authority: authority.pubkey(),
                derivation: authority.pubkey(),
                address_merkle_tree: env.v1_address_trees[0].merkle_tree,
                address_queue: env.v1_address_trees[0].queue,
                changelog_index: 0,
                indexed_changelog_index: 0,
                value: 1,
                low_address_index: 1,
                low_address_value: [0u8; 32],
                low_address_next_index: 1,
                low_address_next_value: [0u8; 32],
                low_address_proof: [[0u8; 32]; 16],
                is_metadata_forester: false,
            },
            0,
        );
        // Swap the derived forester pda with an initialized but invalid one.
        instruction.accounts[0].pubkey =
            get_forester_epoch_pda_from_authority(&env.protocol.forester.pubkey(), 0).0;

        let result = rpc
            .create_and_send_transaction(&[instruction], &authority.pubkey(), &[&authority])
            .await;
        assert_rpc_error(result, 0, RegistryError::InvalidForester.into()).unwrap();
    }
    // 4 FAIL: batch append failed
    {
        let authority = rpc.get_payer().insecure_clone();
        let mut instruction = create_batch_append_instruction(
            authority.pubkey(),
            authority.pubkey(),
            env.v2_state_trees[0].merkle_tree,
            env.v2_state_trees[0].output_queue,
            0,
            Vec::new(),
        );
        // Swap the derived forester pda with an initialized but invalid one.
        instruction.accounts[0].pubkey =
            get_forester_epoch_pda_from_authority(&env.protocol.forester.pubkey(), 0).0;

        let result = rpc
            .create_and_send_transaction(&[instruction], &authority.pubkey(), &[&authority])
            .await;
        assert_rpc_error(result, 0, RegistryError::InvalidForester.into()).unwrap();
    }
    // 4 FAIL: batch nullify failed
    {
        let expected_error_code = RegistryError::InvalidForester.into();
        let authority = rpc.get_payer().insecure_clone();
        let mut instruction = create_batch_nullify_instruction(
            authority.pubkey(),
            authority.pubkey(),
            env.v2_state_trees[0].merkle_tree,
            0,
            Vec::new(),
        );
        // Swap the derived forester pda with an initialized but invalid one.
        instruction.accounts[0].pubkey =
            get_forester_epoch_pda_from_authority(&env.protocol.forester.pubkey(), 0).0;

        let result = rpc
            .create_and_send_transaction(&[instruction], &authority.pubkey(), &[&authority])
            .await;
        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 4 FAIL: batch append failed
    {
        let expected_error_code = RegistryError::InvalidForester.into();
        let authority = rpc.get_payer().insecure_clone();
        let mut instruction = create_batch_append_instruction(
            authority.pubkey(),
            authority.pubkey(),
            env.v2_state_trees[0].merkle_tree,
            env.v2_state_trees[0].output_queue,
            0,
            Vec::new(),
        );
        // Swap the derived forester pda with an initialized but invalid one.
        instruction.accounts[0].pubkey =
            get_forester_epoch_pda_from_authority(&env.protocol.forester.pubkey(), 0).0;

        let result = rpc
            .create_and_send_transaction(&[instruction], &authority.pubkey(), &[&authority])
            .await;
        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 4 FAIL: batch nullify failed
    {
        let expected_error_code = RegistryError::InvalidForester.into();
        let authority = rpc.get_payer().insecure_clone();
        let mut instruction = create_batch_nullify_instruction(
            authority.pubkey(),
            authority.pubkey(),
            env.v2_state_trees[0].merkle_tree,
            0,
            Vec::new(),
        );
        // Swap the derived forester pda with an initialized but invalid one.
        instruction.accounts[0].pubkey =
            get_forester_epoch_pda_from_authority(&env.protocol.forester.pubkey(), 0).0;

        let result = rpc
            .create_and_send_transaction(&[instruction], &authority.pubkey(), &[&authority])
            .await;
        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 5. FAIL: rollover address tree with invalid authority
    {
        let new_queue_keypair = Keypair::new();
        let new_merkle_tree_keypair = Keypair::new();
        let expected_error_code = RegistryError::InvalidForester as u32 + 6000;
        let authority = rpc.get_payer().insecure_clone();
        let mut instructions = create_rollover_address_merkle_tree_instructions(
            &mut rpc,
            &authority.pubkey(),
            &authority.pubkey(),
            &new_queue_keypair,
            &new_merkle_tree_keypair,
            &env.v1_address_trees[0].merkle_tree,
            &env.v1_address_trees[0].queue,
            0, // TODO: adapt epoch
            false,
        )
        .await;
        // Swap the derived forester pda with an initialized but invalid one.
        instructions[2].accounts[0].pubkey =
            get_forester_epoch_pda_from_authority(&env.protocol.forester.pubkey(), 0).0;

        let result = rpc
            .create_and_send_transaction(
                &instructions,
                &authority.pubkey(),
                &[&authority, &new_queue_keypair, &new_merkle_tree_keypair],
            )
            .await;
        assert_rpc_error(result, 2, expected_error_code).unwrap();
    }
    // 6. FAIL: rollover state tree with invalid authority
    {
        let new_nullifier_queue_keypair = Keypair::new();
        let new_state_merkle_tree_keypair = Keypair::new();
        let new_cpi_context = Keypair::new();
        let expected_error_code = RegistryError::InvalidForester as u32 + 6000;
        let authority = rpc.get_payer().insecure_clone();
        let mut instructions = create_rollover_state_merkle_tree_instructions(
            &mut rpc,
            &authority.pubkey(),
            &authority.pubkey(),
            &new_nullifier_queue_keypair,
            &new_state_merkle_tree_keypair,
            &new_cpi_context,
            &env.v1_state_trees[0].merkle_tree,
            &env.v1_state_trees[0].nullifier_queue,
            0, // TODO: adapt epoch
            false,
        )
        .await;
        // Swap the derived forester pda with an initialized but invalid one.
        instructions[3].accounts[0].pubkey =
            get_forester_epoch_pda_from_authority(&env.protocol.forester.pubkey(), 0).0;

        let result = rpc
            .create_and_send_transaction(
                &instructions,
                &authority.pubkey(),
                &[
                    &authority,
                    &new_nullifier_queue_keypair,
                    &new_state_merkle_tree_keypair,
                    &new_cpi_context,
                ],
            )
            .await;
        assert_rpc_error(result, 3, expected_error_code).unwrap();
    }
}

// cargo test-sbf -p registry-test -- --test update_registry_governance_on_testnet update_forester_on_testnet --ignored --nocapture
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn update_forester_on_testnet() {
    let test_accounts = TestAccounts::get_program_test_test_accounts();
    let mut rpc = LightClient::new(LightClientConfig::local_no_indexer())
        .await
        .unwrap();
    rpc.airdrop_lamports(
        &test_accounts.protocol.forester.pubkey(),
        LAMPORTS_PER_SOL * 100,
    )
    .await
    .unwrap();
    let forester_epoch = rpc
        .get_anchor_account::<ForesterPda>(&test_accounts.protocol.registered_forester_pda)
        .await
        .unwrap()
        .unwrap();
    println!("ForesterEpoch: {:?}", forester_epoch);
    assert_eq!(
        forester_epoch.authority,
        test_accounts.protocol.forester.pubkey()
    );

    let updated_keypair = read_keypair_file("../../target/forester-keypair.json").unwrap();
    println!("updated keypair: {:?}", updated_keypair.pubkey());
    update_test_forester(
        &mut rpc,
        &test_accounts.protocol.forester,
        &test_accounts.protocol.forester.pubkey(),
        Some(&updated_keypair),
        ForesterConfig::default(),
    )
    .await
    .unwrap();
    let forester_epoch = rpc
        .get_anchor_account::<ForesterPda>(&test_accounts.protocol.registered_forester_pda)
        .await
        .unwrap()
        .unwrap();
    println!("ForesterEpoch: {:?}", forester_epoch);
    assert_eq!(forester_epoch.authority, updated_keypair.pubkey());
}

#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn update_registry_governance_on_testnet() {
    let test_accounts = TestAccounts::get_program_test_test_accounts();
    let mut rpc = LightClient::new(LightClientConfig::local_no_indexer())
        .await
        .unwrap();
    rpc.airdrop_lamports(
        &test_accounts.protocol.governance_authority.pubkey(),
        LAMPORTS_PER_SOL * 100,
    )
    .await
    .unwrap();
    let governance_authority = rpc
        .get_anchor_account::<ProtocolConfigPda>(&test_accounts.protocol.governance_authority_pda)
        .await
        .unwrap()
        .unwrap();
    println!("GroupAuthority: {:?}", governance_authority);
    assert_eq!(
        governance_authority.authority,
        test_accounts.protocol.governance_authority.pubkey()
    );

    let updated_keypair =
        read_keypair_file("../../target/governance-authority-keypair.json").unwrap();
    println!("updated keypair: {:?}", updated_keypair.pubkey());
    let instruction = light_registry::instruction::UpdateProtocolConfig {
        protocol_config: None,
    };
    let accounts = light_registry::accounts::UpdateProtocolConfig {
        protocol_config_pda: test_accounts.protocol.governance_authority_pda,
        authority: test_accounts.protocol.governance_authority.pubkey(),
        new_authority: Some(updated_keypair.pubkey()),
        fee_payer: test_accounts.protocol.governance_authority.pubkey(),
    };
    let ix = Instruction {
        program_id: light_registry::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction.data(),
    };
    let signature = rpc
        .create_and_send_transaction(
            &[ix],
            &test_accounts.protocol.governance_authority.pubkey(),
            &[&test_accounts.protocol.governance_authority],
        )
        .await
        .unwrap();
    println!("signature: {:?}", signature);
    let governance_authority = rpc
        .get_anchor_account::<ProtocolConfigPda>(&test_accounts.protocol.governance_authority_pda)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(governance_authority.authority, updated_keypair.pubkey());
}

// cargo test-sbf -p registry-test -- --test init_accounts --ignored --nocapture
// TODO: refactor into xtask
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn init_accounts() {
    let keypairs = from_target_folder();
    println!(
        "authority pubkey: {:?}",
        keypairs.governance_authority.pubkey()
    );
    println!("forester pubkey: {:?}", keypairs.forester.pubkey());
    setup_accounts(keypairs, RpcUrl::Localnet).await.unwrap();
}

/// Tests:
/// 1. Functional: migrate state
/// 2. Failing - Invalid authority
/// 3. Failing - Invalid merkle tree
/// 4. Failing - Invalid output queue
/// 5. Failing - Invalid derivation
/// 6. Failing - Failing - invoke account compression program migrate state without registered program PDA
///
#[serial]
#[tokio::test]
async fn test_migrate_state() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::default_with_batched_trees(true))
        .await
        .unwrap();

    rpc.indexer = None;
    let test_accounts = rpc.test_accounts.clone();
    let payer = rpc.get_payer().insecure_clone();
    let mut test_indexer: TestIndexer = TestIndexer::init_from_acounts(
        &rpc.test_accounts.protocol.forester.insecure_clone(),
        &rpc.test_accounts,
        ProgramTestConfig::default_with_batched_trees(true)
            .v2_state_tree_config
            .unwrap()
            .output_queue_batch_size as usize,
    )
    .await;
    for _ in 0..4 {
        light_test_utils::system_program::compress_sol_test(
            &mut rpc,
            &mut test_indexer,
            &payer,
            &[],
            false,
            1_000_000,
            &test_accounts.v1_state_trees[0].merkle_tree,
            None,
        )
        .await
        .unwrap();
    }
    // 1. Functional: migrate state
    {
        let merkle_tree =
            get_concurrent_merkle_tree::<StateMerkleTreeAccount, LightProgramTest, Poseidon, 26>(
                &mut rpc,
                test_accounts.v1_state_trees[0].merkle_tree,
            )
            .await;
        let compressed_account =
            &test_indexer.get_compressed_accounts_with_merkle_context_by_owner(&payer.pubkey())[0];
        let hash = compressed_account.hash().unwrap();
        let bundle = &test_indexer
            .get_state_merkle_trees()
            .iter()
            .find(|b| {
                b.accounts.merkle_tree.to_bytes()
                    == compressed_account
                        .merkle_context
                        .merkle_tree_pubkey
                        .to_bytes()
            })
            .unwrap();
        assert_eq!(merkle_tree.root(), bundle.merkle_tree.root());
        let leaf_index = compressed_account.merkle_context.leaf_index as u64;
        let merkle_proof = bundle
            .merkle_tree
            .get_proof_of_leaf(leaf_index as usize, false)
            .unwrap();
        let merkle_leaf = bundle.merkle_tree.get_leaf(leaf_index as usize).unwrap();
        assert_eq!(merkle_leaf, hash);

        let inputs = MigrateLeafParams {
            change_log_index: merkle_tree.changelog_index() as u64,
            leaf: hash,
            leaf_index,
            proof: merkle_proof.try_into().unwrap(),
        };
        let params = CreateMigrateStateInstructionInputs {
            authority: test_accounts.protocol.forester.pubkey(),
            merkle_tree: test_accounts.v1_state_trees[0].merkle_tree,
            output_queue: test_accounts.v2_state_trees[0].output_queue,
            derivation: test_accounts.protocol.forester.pubkey(),
            inputs,
            is_metadata_forester: false,
        };

        let instruction = create_migrate_state_instruction(params, 0);
        rpc.create_and_send_transaction(
            &[instruction],
            &test_accounts.protocol.forester.pubkey(),
            &[&test_accounts.protocol.forester],
        )
        .await
        .unwrap();
        // assert leaf was nullified and inserted into output queue
        {
            let merkle_tree = get_concurrent_merkle_tree::<
                StateMerkleTreeAccount,
                LightProgramTest,
                Poseidon,
                26,
            >(&mut rpc, test_accounts.v1_state_trees[0].merkle_tree)
            .await;
            let bundle = test_indexer
                .get_state_merkle_trees_mut()
                .iter_mut()
                .find(|b| {
                    b.accounts.merkle_tree.to_bytes()
                        == compressed_account
                            .merkle_context
                            .merkle_tree_pubkey
                            .to_bytes()
                })
                .unwrap();
            bundle
                .merkle_tree
                .update(&[0u8; 32], leaf_index as usize)
                .unwrap();
            assert_eq!(merkle_tree.root(), bundle.merkle_tree.root(),);
            let get_leaf = bundle.merkle_tree.get_leaf(leaf_index as usize).unwrap();
            assert_eq!(get_leaf, [0u8; 32]);
            let mut output_queue_account = rpc
                .get_account(test_accounts.v2_state_trees[0].output_queue)
                .await
                .unwrap()
                .unwrap();
            let output_queue =
                BatchedQueueAccount::output_from_bytes(output_queue_account.data_as_mut_slice())
                    .unwrap();
            assert_eq!(output_queue.value_vecs[0][0], hash);
        }
    }
    let instruction_params = {
        let merkle_tree =
            get_concurrent_merkle_tree::<StateMerkleTreeAccount, LightProgramTest, Poseidon, 26>(
                &mut rpc,
                test_accounts.v1_state_trees[0].merkle_tree,
            )
            .await;
        let compressed_account =
            &test_indexer.get_compressed_accounts_with_merkle_context_by_owner(&payer.pubkey())[1];
        let hash = compressed_account.hash().unwrap();
        let bundle = &test_indexer
            .get_state_merkle_trees()
            .iter()
            .find(|b| {
                b.accounts.merkle_tree.to_bytes()
                    == compressed_account
                        .merkle_context
                        .merkle_tree_pubkey
                        .to_bytes()
            })
            .unwrap();
        assert_eq!(merkle_tree.root(), bundle.merkle_tree.root());
        let leaf_index = compressed_account.merkle_context.leaf_index as u64;
        let merkle_proof = bundle
            .merkle_tree
            .get_proof_of_leaf(leaf_index as usize, false)
            .unwrap();
        let merkle_leaf = bundle.merkle_tree.get_leaf(leaf_index as usize).unwrap();
        assert_eq!(merkle_leaf, hash);

        let inputs = MigrateLeafParams {
            change_log_index: merkle_tree.changelog_index() as u64,
            leaf: hash,
            leaf_index,
            proof: merkle_proof.try_into().unwrap(),
        };
        CreateMigrateStateInstructionInputs {
            authority: test_accounts.protocol.forester.pubkey(),
            merkle_tree: test_accounts.v1_state_trees[0].merkle_tree,
            output_queue: test_accounts.v2_state_trees[0].output_queue,
            derivation: test_accounts.protocol.forester.pubkey(),
            inputs,
            is_metadata_forester: false,
        }
    };
    // 2. Failing - invalid authority
    {
        let mut params = instruction_params.clone();
        params.authority = payer.pubkey();
        let instruction = create_migrate_state_instruction(params, 0);
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await;
        assert_rpc_error(result, 0, RegistryError::InvalidForester.into()).unwrap();
    }
    // 3. Failing - invalid output queue
    {
        let mut params = instruction_params.clone();
        params.output_queue = test_accounts.v1_state_trees[0].nullifier_queue;
        let instruction = create_migrate_state_instruction(params, 0);
        let result = rpc
            .create_and_send_transaction(
                &[instruction],
                &test_accounts.protocol.forester.pubkey(),
                &[&test_accounts.protocol.forester],
            )
            .await;
        assert_rpc_error(result, 0, AccountError::InvalidDiscriminator.into()).unwrap();
    }
    // 4. Failing - invalid state Merkle tree
    {
        let mut params = instruction_params.clone();
        params.merkle_tree = test_accounts.v1_address_trees[0].merkle_tree;
        let instruction = create_migrate_state_instruction(params, 0);
        let result = rpc
            .create_and_send_transaction(
                &[instruction],
                &test_accounts.protocol.forester.pubkey(),
                &[&test_accounts.protocol.forester],
            )
            .await;
        assert_rpc_error(
            result,
            0,
            anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch.into(),
        )
        .unwrap();
    }
    // 5. Failing - invalid derivation
    {
        let mut params = instruction_params.clone();
        params.derivation = payer.pubkey();
        let instruction = create_migrate_state_instruction(params, 0);
        let result = rpc
            .create_and_send_transaction(
                &[instruction],
                &test_accounts.protocol.forester.pubkey(),
                &[&test_accounts.protocol.forester],
            )
            .await;
        assert_rpc_error(
            result,
            0,
            anchor_lang::error::ErrorCode::AccountNotInitialized.into(),
        )
        .unwrap();
    }
    // 6. Failing - invoke account compression program migrate state without registered program PDA
    {
        let instruction = account_compression::instruction::MigrateState {
            _input: instruction_params.inputs,
        };
        let accounts = account_compression::accounts::MigrateState {
            authority: payer.pubkey(),
            merkle_tree: instruction_params.merkle_tree,
            output_queue: instruction_params.output_queue,
            registered_program_pda: None,
            log_wrapper: NOOP_PROGRAM_ID,
        };
        let ix = Instruction {
            program_id: account_compression::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
            .await;
        assert_rpc_error(
            result,
            0,
            AccountCompressionErrorCode::RegistryProgramIsNone.into(),
        )
        .unwrap();
    }
}
/// Test:
/// 1. Failing: rollover with invalid forester
/// 2. Functional: rollover with network fee
/// 3. Functional: rollover without network fee and custom forester
/// 4. failing: create with state tree with custom forester and invalid non-zero network fee
#[serial]
#[tokio::test]
async fn test_rollover_batch_state_tree() {
    {
        let mut params = InitStateTreeAccountsInstructionData::test_default();
        params.rollover_threshold = Some(0);
        let is_light_forester = true;
        let mut config = ProgramTestConfig::default_with_batched_trees(false);
        config.v2_state_tree_config = Some(params);

        let mut rpc = LightProgramTest::new(config).await.unwrap();

        rpc.indexer = None;
        let test_accounts = rpc.test_accounts.clone();
        let payer = rpc.get_payer().insecure_clone();
        let mut test_indexer: TestIndexer = TestIndexer::init_from_acounts(
            &test_accounts.protocol.forester.insecure_clone(),
            &test_accounts,
            50,
        )
        .await;
        light_test_utils::system_program::compress_sol_test(
            &mut rpc,
            &mut test_indexer,
            &payer,
            &[],
            false,
            1_000_000,
            &test_accounts.v2_state_trees[0].output_queue,
            None,
        )
        .await
        .unwrap();
        let new_merkle_tree_keypair = Keypair::new();
        let new_nullifier_queue_keypair = Keypair::new();
        let new_cpi_context = Keypair::new();

        // 1. failing invalid forester
        {
            let unregistered_forester_keypair = Keypair::new();
            rpc.airdrop_lamports(&unregistered_forester_keypair.pubkey(), 1_000_000_000)
                .await
                .unwrap();

            let result = perform_rollover_batch_state_merkle_tree(
                &mut rpc,
                &payer,
                test_accounts.protocol.forester.pubkey(),
                test_accounts.v2_state_trees[0].merkle_tree,
                test_accounts.v2_state_trees[0].output_queue,
                &new_merkle_tree_keypair,
                &new_nullifier_queue_keypair,
                &new_cpi_context,
                0,
                is_light_forester,
            )
            .await;

            assert_rpc_error(result, 3, RegistryError::InvalidForester.into()).unwrap();
        }

        // 2. functional with network fee
        {
            perform_rollover_batch_state_merkle_tree(
                &mut rpc,
                &test_accounts.protocol.forester,
                test_accounts.protocol.forester.pubkey(),
                test_accounts.v2_state_trees[0].merkle_tree,
                test_accounts.v2_state_trees[0].output_queue,
                &new_merkle_tree_keypair,
                &new_nullifier_queue_keypair,
                &new_cpi_context,
                0,
                is_light_forester,
            )
            .await
            .unwrap();
            let new_cpi_ctx_account = rpc
                .get_account(new_cpi_context.pubkey())
                .await
                .unwrap()
                .unwrap();
            assert_perform_state_mt_roll_over(
                &mut rpc,
                test_accounts.protocol.group_pda,
                test_accounts.v2_state_trees[0].merkle_tree,
                new_merkle_tree_keypair.pubkey(),
                test_accounts.v2_state_trees[0].output_queue,
                new_nullifier_queue_keypair.pubkey(),
                params,
                new_cpi_ctx_account.lamports,
            )
            .await;
        }
    }
    {
        let custom_forester = Keypair::new();
        let mut params = InitStateTreeAccountsInstructionData::test_default();
        params.rollover_threshold = Some(0);
        params.forester = Some(custom_forester.pubkey().into());
        params.network_fee = None;

        let mut tree_params = InitAddressTreeAccountsInstructionData::test_default();
        tree_params.rollover_threshold = Some(0);
        let mut config = ProgramTestConfig::default_with_batched_trees(false);
        config.v2_state_tree_config = Some(params);
        config.v2_address_tree_config = Some(tree_params);
        let result = LightProgramTest::new(config).await;

        assert_rpc_error(
            result,
            3,
            light_registry::errors::RegistryError::InvalidNetworkFee.into(),
        )
        .unwrap();
    }
}

#[serial]
#[tokio::test]
async fn test_batch_address_tree() {
    let mut config = ProgramTestConfig::default_test_forester(true);
    let tree_params = config.v2_address_tree_config.unwrap();
    config.v2_state_tree_config = Some(InitStateTreeAccountsInstructionData::default());
    config.additional_programs = Some(vec![(
        "create_address_test_program",
        CREATE_ADDRESS_TEST_PROGRAM_ID,
    )]);
    let mut rpc = LightProgramTest::new(config).await.unwrap();

    rpc.indexer = None;
    let env = rpc.test_accounts.clone();

    let payer = rpc.get_payer().insecure_clone();
    let mut test_indexer = TestIndexer::init_from_acounts(&payer, &env, 50).await;
    {
        let new_merkle_tree = Keypair::new();
        let test_tree_params = InitAddressTreeAccountsInstructionData {
            network_fee: Some(1),
            ..Default::default()
        };
        let result =
            create_batch_address_merkle_tree(&mut rpc, &payer, &new_merkle_tree, test_tree_params)
                .await;
        assert_rpc_error(result, 1, RegistryError::InvalidNetworkFee.into()).unwrap();
    }

    for i in 0..tree_params.input_queue_batch_size * 2 {
        println!("tx {}", i);
        perform_create_pda_with_event_rnd(&mut test_indexer, &mut rpc, &env, &payer)
            .await
            .unwrap();
    }
    {
        println!("pre perform_batch_address_merkle_tree_update");
        for _ in 0..1 {
            perform_batch_address_merkle_tree_update(
                &mut rpc,
                &mut test_indexer,
                &env.protocol.forester,
                &env.protocol.forester.pubkey(),
                &env.v2_address_trees[0],
                0,
            )
            .await
            .unwrap();
            let mut account = rpc
                .get_account(env.v2_address_trees[0])
                .await
                .unwrap()
                .unwrap();
            test_indexer
                .finalize_batched_address_tree_update(
                    env.v2_address_trees[0],
                    account.data.as_mut_slice(),
                )
                .await;
        }
    }

    {
        println!("pre perform_batch_address_merkle_tree_update");
        for _ in 0..6 {
            perform_batch_address_merkle_tree_update(
                &mut rpc,
                &mut test_indexer,
                &env.protocol.forester,
                &env.protocol.forester.pubkey(),
                &env.v2_address_trees[0],
                0,
            )
            .await
            .unwrap();
            let mut account = rpc
                .get_account(env.v2_address_trees[0])
                .await
                .unwrap()
                .unwrap();
            test_indexer
                .finalize_batched_address_tree_update(
                    env.v2_address_trees[0],
                    account.data.as_mut_slice(),
                )
                .await;
        }
    }

    // Non eligible forester.
    {
        let unregistered_forester_keypair = Keypair::new();
        rpc.airdrop_lamports(&unregistered_forester_keypair.pubkey(), 1_000_000_000)
            .await
            .unwrap();
        let result = perform_batch_address_merkle_tree_update(
            &mut rpc,
            &mut test_indexer,
            &unregistered_forester_keypair,
            &env.protocol.forester.pubkey(),
            &env.v2_address_trees[0],
            0,
        )
        .await;
        assert_rpc_error(result, 0, RegistryError::InvalidForester.into()).unwrap();
    }

    for _ in 0..tree_params.input_queue_batch_size {
        perform_create_pda_with_event_rnd(&mut test_indexer, &mut rpc, &env, &payer)
            .await
            .unwrap();
    }
    for _ in 0..3 {
        perform_batch_address_merkle_tree_update(
            &mut rpc,
            &mut test_indexer,
            &env.protocol.forester,
            &env.protocol.forester.pubkey(),
            &env.v2_address_trees[0],
            0,
        )
        .await
        .unwrap();
        let mut account = rpc
            .get_account(env.v2_address_trees[0])
            .await
            .unwrap()
            .unwrap();
        test_indexer
            .finalize_batched_address_tree_update(
                env.v2_address_trees[0],
                account.data.as_mut_slice(),
            )
            .await;
    }
    let mut account = rpc
        .get_account(env.v2_address_trees[0])
        .await
        .unwrap()
        .unwrap();
    test_indexer
        .finalize_batched_address_tree_update(env.v2_address_trees[0], account.data.as_mut_slice())
        .await;
}

pub async fn perform_batch_address_merkle_tree_update<
    R: Rpc,
    I: Indexer + TestIndexerExtensions,
>(
    rpc: &mut R,
    test_indexer: &mut I,
    forester: &Keypair,
    derivation_pubkey: &Pubkey,
    merkle_tree_pubkey: &Pubkey,
    epoch: u64,
) -> Result<Signature, RpcError> {
    let instruction_data = create_batch_update_address_tree_instruction_data_with_proof(
        rpc,
        test_indexer,
        *merkle_tree_pubkey,
    )
    .await
    .unwrap();

    let instruction = create_batch_update_address_tree_instruction(
        forester.pubkey(),
        *derivation_pubkey,
        *merkle_tree_pubkey,
        epoch,
        instruction_data.try_to_vec().unwrap(),
    );
    rpc.create_and_send_transaction(&[instruction], &forester.pubkey(), &[forester])
        .await
}

#[serial]
#[tokio::test]
async fn test_rollover_batch_address_tree() {
    let mut tree_params = InitAddressTreeAccountsInstructionData::test_default();
    tree_params.rollover_threshold = Some(0);
    let mut config = ProgramTestConfig::default_with_batched_trees(true);
    config.additional_programs = Some(vec![(
        "create_address_test_program",
        CREATE_ADDRESS_TEST_PROGRAM_ID,
    )]);
    config.v2_address_tree_config = Some(tree_params);
    let mut rpc = LightProgramTest::new(config).await.unwrap();

    rpc.indexer = None;
    let env = rpc.test_accounts.clone();

    let payer = rpc.get_payer().insecure_clone();
    let mut test_indexer = TestIndexer::init_from_acounts(&payer, &env, 50).await;
    // Create one address to pay for rollover fees.
    perform_create_pda_with_event_rnd(&mut test_indexer, &mut rpc, &env, &payer)
        .await
        .unwrap();
    let new_merkle_tree_keypair = Keypair::new();
    perform_rollover_batch_address_merkle_tree(
        &mut rpc,
        &env.protocol.forester,
        env.protocol.forester.pubkey(),
        env.v2_address_trees[0],
        &new_merkle_tree_keypair,
        0,
    )
    .await
    .unwrap();
    let mut account = rpc
        .get_account(new_merkle_tree_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let mt_params = CreateTreeParams::from_address_ix_params(
        tree_params,
        env.protocol.group_pda.into(),
        new_merkle_tree_keypair.pubkey().into(),
    );
    let zero_copy_account =
        BatchedMerkleTreeMetadata::new_address_tree(mt_params, account.lamports);
    assert_address_mt_zero_copy_initialized(
        &mut account.data,
        zero_copy_account,
        &new_merkle_tree_keypair.pubkey().into(),
    );
    // Create one address to pay for rollover fees.
    perform_create_pda_with_event_rnd(&mut test_indexer, &mut rpc, &env, &payer)
        .await
        .unwrap();
    // invalid forester
    {
        let unregistered_forester_keypair = Keypair::new();
        rpc.airdrop_lamports(&unregistered_forester_keypair.pubkey(), 1_000_000_000)
            .await
            .unwrap();
        let new_merkle_tree_keypair2 = Keypair::new();

        let result = perform_rollover_batch_address_merkle_tree(
            &mut rpc,
            &unregistered_forester_keypair,
            env.protocol.forester.pubkey(),
            new_merkle_tree_keypair.pubkey(),
            &new_merkle_tree_keypair2,
            0,
        )
        .await;
        assert_rpc_error(result, 1, RegistryError::InvalidForester.into()).unwrap();
    }
    airdrop_lamports(&mut rpc, &new_merkle_tree_keypair.pubkey(), 100_000_000_000)
        .await
        .unwrap();
    let new_merkle_tree_keypair2 = Keypair::new();
    perform_rollover_batch_address_merkle_tree(
        &mut rpc,
        &env.protocol.forester,
        env.protocol.forester.pubkey(),
        new_merkle_tree_keypair.pubkey(),
        &new_merkle_tree_keypair2,
        0,
    )
    .await
    .unwrap();
}

#[ignore = "requires account compression program without test features"]
#[tokio::test]
async fn test_v2_tree_mainnet_init() {
    let mut config = ProgramTestConfig::default_test_forester(true);
    config.v2_state_tree_config = Some(InitStateTreeAccountsInstructionData::default());
    config.v2_address_tree_config = Some(InitAddressTreeAccountsInstructionData::default());
    config.additional_programs = Some(vec![(
        "create_address_test_program",
        CREATE_ADDRESS_TEST_PROGRAM_ID,
    )]);
    LightProgramTest::new(config).await.unwrap();
}

#[ignore = "requires account compression program without test features"]
#[tokio::test]
async fn test_v2_state_tree_mainnet_init_fail() {
    let mut config = ProgramTestConfig::default_test_forester(true);
    config.v2_state_tree_config = Some(InitStateTreeAccountsInstructionData::test_default());
    config.v1_state_tree_config = StateMerkleTreeConfig::default();
    config.v2_address_tree_config = Some(InitAddressTreeAccountsInstructionData::default());
    config.additional_programs = Some(vec![(
        "create_address_test_program",
        CREATE_ADDRESS_TEST_PROGRAM_ID,
    )]);
    let result = LightProgramTest::new(config).await;
    assert_rpc_error(
        result,
        3,
        AccountCompressionErrorCode::UnsupportedParameters.into(),
    )
    .unwrap();
}

#[ignore = "requires account compression program without test features"]
#[tokio::test]
async fn test_v2_address_tree_mainnet_init_fail() {
    let mut config = ProgramTestConfig::default_test_forester(true);
    config.v2_state_tree_config = Some(InitStateTreeAccountsInstructionData::default());
    config.v2_address_tree_config = Some(InitAddressTreeAccountsInstructionData::test_default());
    config.additional_programs = Some(vec![(
        "create_address_test_program",
        CREATE_ADDRESS_TEST_PROGRAM_ID,
    )]);
    let result = LightProgramTest::new(config).await;
    assert_rpc_error(
        result,
        1,
        AccountCompressionErrorCode::UnsupportedParameters.into(),
    )
    .unwrap();
}
