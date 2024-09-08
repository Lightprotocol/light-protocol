#![cfg(feature = "test-sbf")]

use account_compression::{
    AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig, StateMerkleTreeConfig,
};
use anchor_lang::{InstructionData, ToAccountMetas};
use light_registry::account_compression_cpi::sdk::{
    create_nullify_instruction, create_update_address_merkle_tree_instruction,
    CreateNullifyInstructionInputs, UpdateAddressMerkleTreeInstructionInputs,
};
use light_registry::errors::RegistryError;
use light_registry::protocol_config::state::{ProtocolConfig, ProtocolConfigPda};
use light_registry::sdk::{
    create_finalize_registration_instruction, create_report_work_instruction,
    create_update_forester_pda_weight_instruction,
};
use light_registry::utils::{
    get_cpi_authority_pda, get_forester_epoch_pda_from_authority, get_forester_pda,
    get_protocol_config_pda_address,
};
use light_registry::{ForesterConfig, ForesterEpochPda, ForesterPda};
use light_test_utils::assert_epoch::{
    assert_epoch_pda, assert_finalized_epoch_registration, assert_registered_forester_pda,
    assert_report_work, fetch_epoch_and_forester_pdas,
};
use light_test_utils::e2e_test_env::init_program_test_env;
use light_test_utils::rpc::ProgramTestRpcConnection;
use light_test_utils::test_env::{
    create_address_merkle_tree_and_queue_account, create_state_merkle_tree_and_queue_account,
    deregister_program_with_registry_program, initialize_new_group,
    register_program_with_registry_program, setup_accounts, setup_test_programs,
    setup_test_programs_with_accounts_with_protocol_config, EnvAccountKeypairs,
    GROUP_PDA_SEED_TEST_KEYPAIR, OLD_REGISTRY_ID_TEST_KEYPAIR,
};
use light_test_utils::test_env::{get_test_env_accounts, setup_test_programs_with_accounts};
use light_test_utils::test_forester::{empty_address_queue_test, nullify_compressed_accounts};
use light_test_utils::{
    assert_rpc_error, create_rollover_address_merkle_tree_instructions,
    create_rollover_state_merkle_tree_instructions, register_test_forester, update_test_forester,
    Epoch, RpcConnection, SolanaRpcConnection, SolanaRpcUrl, TreeAccounts, TreeType,
};
use solana_sdk::{
    instruction::Instruction,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
};

#[tokio::test]
async fn test_initialize_protocol_config() {
    let rpc = setup_test_programs(None).await;
    let mut rpc = ProgramTestRpcConnection { context: rpc };

    let payer = rpc.get_payer().insecure_clone();
    let program_account_keypair = Keypair::from_bytes(&OLD_REGISTRY_ID_TEST_KEYPAIR).unwrap();
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

    let group_seed_keypair = Keypair::from_bytes(&GROUP_PDA_SEED_TEST_KEYPAIR).unwrap();
    let group_pda =
        initialize_new_group(&group_seed_keypair, &payer, &mut rpc, cpi_authority_pda.0).await;

    let random_program_keypair = Keypair::new();
    // register program with invalid authority
    {
        let result = register_program_with_registry_program(
            &mut rpc,
            &payer,
            &group_pda,
            &random_program_keypair.pubkey(),
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
            &random_program_keypair.pubkey(),
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
    // register program
    {
        // let program_keypair = Keypair::from_bytes(light_registry::id().as_ref()).unwrap();
        register_program_with_registry_program(
            &mut rpc,
            &updated_keypair,
            &group_pda,
            &light_registry::id(),
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
        create_address_merkle_tree_and_queue_account(
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
        .await
        .unwrap();
    }
    // initialize a Merkle tree with network fee = 5000 (default)
    {
        let merkle_tree_keypair = Keypair::new();
        let queue_keypair = Keypair::new();
        create_address_merkle_tree_and_queue_account(
            &payer,
            true,
            &mut rpc,
            &merkle_tree_keypair,
            &queue_keypair,
            None,
            None,
            &AddressMerkleTreeConfig {
                network_fee: Some(5000),
                ..Default::default()
            },
            &AddressQueueConfig::default(),
            0,
        )
        .await
        .unwrap();
    }
    // FAIL: initialize a Merkle tree with network fee != 0 || 5000
    {
        let merkle_tree_keypair = Keypair::new();
        let queue_keypair = Keypair::new();
        let result = create_address_merkle_tree_and_queue_account(
            &payer,
            true,
            &mut rpc,
            &merkle_tree_keypair,
            &queue_keypair,
            None,
            None,
            &AddressMerkleTreeConfig {
                network_fee: Some(5001),
                ..Default::default()
            },
            &AddressQueueConfig::default(),
            0,
        )
        .await;
        let expected_error_code = RegistryError::InvalidNetworkFee as u32 + 6000;
        assert_rpc_error(result, 2, expected_error_code).unwrap();
    }
}

#[tokio::test]
async fn test_custom_forester() {
    let (mut rpc, env) = setup_test_programs_with_accounts_with_protocol_config(
        None,
        ProtocolConfig::default(),
        false,
    )
    .await;
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
            let mut e2e_env = init_program_test_env(rpc, &env).await;
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
/// Test:
/// 1. SUCCESS: Register a forester
/// 2. SUCCESS: Update forester authority
/// 3. SUCCESS: Register forester for epoch
#[tokio::test]
async fn test_register_and_update_forester_pda() {
    let (mut rpc, env) = setup_test_programs_with_accounts_with_protocol_config(
        None,
        ProtocolConfig::default(),
        false,
    )
    .await;
    let forester_keypair = Keypair::new();
    rpc.airdrop_lamports(&forester_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let config = ForesterConfig { fee: 1 };
    // 1. SUCCESS: Register a forester
    register_test_forester(
        &mut rpc,
        &env.governance_authority,
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
        .get_anchor_account::<ProtocolConfigPda>(&env.governance_authority_pda)
        .await
        .unwrap()
        .unwrap()
        .config;

    // SUCCESS: update forester weight
    {
        let ix = create_update_forester_pda_weight_instruction(
            &forester_keypair.pubkey(),
            &env.governance_authority.pubkey(),
            11,
        );
        rpc.create_and_send_transaction(
            &[ix],
            &env.governance_authority.pubkey(),
            &[&env.governance_authority],
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
            &env.governance_authority.pubkey(),
            1,
        );
        rpc.create_and_send_transaction(
            &[ix],
            &env.governance_authority.pubkey(),
            &[&env.governance_authority],
        )
        .await
        .unwrap();
    }

    // 3. SUCCESS: register forester for epoch
    let tree_accounts = vec![
        TreeAccounts {
            tree_type: TreeType::State,
            merkle_tree: env.merkle_tree_pubkey,
            queue: env.nullifier_queue_pubkey,
            is_rolledover: false,
        },
        TreeAccounts {
            tree_type: TreeType::Address,
            merkle_tree: env.address_merkle_tree_pubkey,
            queue: env.address_merkle_tree_queue_pubkey,
            is_rolledover: false,
        },
    ];

    let registered_epoch = Epoch::register(&mut rpc, &protocol_config, &forester_keypair)
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
        .await
        .unwrap();
    // finalize registration
    {
        registered_epoch
            .fetch_account_and_add_trees_with_schedule(&mut rpc, &tree_accounts)
            .await
            .unwrap();
        let ix = create_finalize_registration_instruction(
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
        let mut e2e_env = init_program_test_env(rpc, &env).await;
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
    .await
    .unwrap();
    // register for next epoch
    let next_registered_epoch = Epoch::register(&mut rpc, &protocol_config, &forester_keypair)
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
        .await
        .unwrap();
    // report work
    {
        let (pre_forester_epoch_pda, pre_epoch_pda) = fetch_epoch_and_forester_pdas(
            &mut rpc,
            &registered_epoch.forester_epoch_pda,
            &registered_epoch.epoch_pda,
        )
        .await;
        let ix = create_report_work_instruction(&forester_keypair.pubkey(), registered_epoch.epoch);
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
#[tokio::test]
async fn failing_test_forester() {
    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let payer = rpc.get_payer().insecure_clone();
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
        let forester_pda = get_forester_pda(&env.forester.pubkey()).0;
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
        println!("here1");
    }
    // 3. FAIL: Update forester pda weight with invalid authority
    {
        let ix = light_registry::instruction::UpdateForesterPdaWeight { new_weight: 11 };
        let accounts = light_registry::accounts::UpdateForesterPdaWeight {
            forester_pda: get_forester_pda(&env.forester.pubkey()).0,
            authority: payer.pubkey(),
            protocol_config_pda: env.governance_authority_pda,
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
        println!("here1");
    }
    // 4. FAIL: Nullify with invalid authority
    {
        let expected_error_code = RegistryError::InvalidForester as u32 + 6000;
        let inputs = CreateNullifyInstructionInputs {
            authority: payer.pubkey(),
            nullifier_queue: env.nullifier_queue_pubkey,
            merkle_tree: env.merkle_tree_pubkey,
            change_log_indices: vec![1],
            leaves_queue_indices: vec![1u16],
            indices: vec![0u64],
            proofs: vec![vec![[0u8; 32]; 26]],
            derivation: payer.pubkey(),
            is_metadata_forester: false,
        };
        let mut ix = create_nullify_instruction(inputs, 0);
        // Swap the derived forester pda with an initialized but invalid one.
        ix.accounts[0].pubkey = get_forester_epoch_pda_from_authority(&env.forester.pubkey(), 0).0;
        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
            .await;
        assert_rpc_error(result, 0, expected_error_code).unwrap();
        println!("here1");
    }
    // 4 FAIL: update address Merkle tree failed
    {
        let expected_error_code = RegistryError::InvalidForester as u32 + 6000;
        let authority = rpc.get_payer().insecure_clone();
        let mut instruction = create_update_address_merkle_tree_instruction(
            UpdateAddressMerkleTreeInstructionInputs {
                authority: authority.pubkey(),
                address_merkle_tree: env.address_merkle_tree_pubkey,
                address_queue: env.address_merkle_tree_queue_pubkey,
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
            get_forester_epoch_pda_from_authority(&env.forester.pubkey(), 0).0;
        println!("here1");

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
            &new_queue_keypair,
            &new_merkle_tree_keypair,
            &env.address_merkle_tree_pubkey,
            &env.address_merkle_tree_queue_pubkey,
            0, // TODO: adapt epoch
            false,
        )
        .await;
        // Swap the derived forester pda with an initialized but invalid one.
        instructions[2].accounts[0].pubkey =
            get_forester_epoch_pda_from_authority(&env.forester.pubkey(), 0).0;

        let result = rpc
            .create_and_send_transaction(
                &instructions,
                &authority.pubkey(),
                &[&authority, &new_queue_keypair, &new_merkle_tree_keypair],
            )
            .await;
        assert_rpc_error(result, 2, expected_error_code).unwrap();
        println!("here1");
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
            &new_nullifier_queue_keypair,
            &new_state_merkle_tree_keypair,
            &new_cpi_context,
            &env.merkle_tree_pubkey,
            &env.nullifier_queue_pubkey,
            0, // TODO: adapt epoch
            false,
        )
        .await;
        // Swap the derived forester pda with an initialized but invalid one.
        instructions[3].accounts[0].pubkey =
            get_forester_epoch_pda_from_authority(&env.forester.pubkey(), 0).0;

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
    let env_accounts = get_test_env_accounts();
    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::ZKTestnet, None);
    rpc.airdrop_lamports(&env_accounts.forester.pubkey(), LAMPORTS_PER_SOL * 100)
        .await
        .unwrap();
    let forester_epoch = rpc
        .get_anchor_account::<ForesterPda>(&env_accounts.registered_forester_pda)
        .await
        .unwrap()
        .unwrap();
    println!("ForesterEpoch: {:?}", forester_epoch);
    assert_eq!(forester_epoch.authority, env_accounts.forester.pubkey());

    let updated_keypair = read_keypair_file("../../target/forester-keypair.json").unwrap();
    println!("updated keypair: {:?}", updated_keypair.pubkey());
    update_test_forester(
        &mut rpc,
        &env_accounts.forester,
        &env_accounts.forester.pubkey(),
        Some(&updated_keypair),
        ForesterConfig::default(),
    )
    .await
    .unwrap();
    let forester_epoch = rpc
        .get_anchor_account::<ForesterPda>(&env_accounts.registered_forester_pda)
        .await
        .unwrap()
        .unwrap();
    println!("ForesterEpoch: {:?}", forester_epoch);
    assert_eq!(forester_epoch.authority, updated_keypair.pubkey());
}

#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn update_registry_governance_on_testnet() {
    let env_accounts = get_test_env_accounts();
    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::ZKTestnet, None);
    rpc.airdrop_lamports(
        &env_accounts.governance_authority.pubkey(),
        LAMPORTS_PER_SOL * 100,
    )
    .await
    .unwrap();
    let governance_authority = rpc
        .get_anchor_account::<ProtocolConfigPda>(&env_accounts.governance_authority_pda)
        .await
        .unwrap()
        .unwrap();
    println!("GroupAuthority: {:?}", governance_authority);
    assert_eq!(
        governance_authority.authority,
        env_accounts.governance_authority.pubkey()
    );

    let updated_keypair =
        read_keypair_file("../../target/governance-authority-keypair.json").unwrap();
    println!("updated keypair: {:?}", updated_keypair.pubkey());
    let instruction = light_registry::instruction::UpdateProtocolConfig {
        protocol_config: None,
    };
    let accounts = light_registry::accounts::UpdateProtocolConfig {
        protocol_config_pda: env_accounts.governance_authority_pda,
        authority: env_accounts.governance_authority.pubkey(),
        new_authority: Some(updated_keypair.pubkey()),
        fee_payer: env_accounts.governance_authority.pubkey(),
    };
    let ix = Instruction {
        program_id: light_registry::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction.data(),
    };
    let signature = rpc
        .create_and_send_transaction(
            &[ix],
            &env_accounts.governance_authority.pubkey(),
            &[&env_accounts.governance_authority],
        )
        .await
        .unwrap();
    println!("signature: {:?}", signature);
    let governance_authority = rpc
        .get_anchor_account::<ProtocolConfigPda>(&env_accounts.governance_authority_pda)
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
    let keypairs = EnvAccountKeypairs::from_target_folder();
    println!(
        "authority pubkey: {:?}",
        keypairs.governance_authority.pubkey()
    );
    println!("forester pubkey: {:?}", keypairs.forester.pubkey());
    setup_accounts(keypairs, SolanaRpcUrl::Localnet).await;
}
