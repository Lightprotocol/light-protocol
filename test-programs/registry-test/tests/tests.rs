#![cfg(feature = "test-sbf")]

use anchor_lang::{InstructionData, ToAccountMetas};
use light_registry::account_compression_cpi::sdk::{
    create_nullify_instruction, create_update_address_merkle_tree_instruction,
    CreateNullifyInstructionInputs, UpdateAddressMerkleTreeInstructionInputs,
};
use light_registry::errors::RegistryError;
use light_registry::protocol_config::state::{ProtocolConfig, ProtocolConfigPda};
use light_registry::sdk::{
    create_finalize_registration_instruction, create_report_work_instruction,
};
use light_registry::utils::{get_forester_epoch_pda_address, get_protocol_config_pda_address};
use light_registry::{ForesterAccount, ForesterConfig, ForesterEpochPda};
use light_test_utils::assert_epoch::{
    assert_epoch_pda, assert_finalized_epoch_registration, assert_registered_forester_pda,
    assert_report_work, fetch_epoch_and_forester_pdas,
};
use light_test_utils::e2e_test_env::init_program_test_env;
use light_test_utils::forester_epoch::{Epoch, TreeAccounts, TreeType};
use light_test_utils::rpc::solana_rpc::SolanaRpcUrl;
use light_test_utils::test_env::{
    setup_accounts_devnet, setup_test_programs_with_accounts_with_protocol_config,
};
use light_test_utils::test_forester::{empty_address_queue_test, nullify_compressed_accounts};
use light_test_utils::{
    registry::{
        create_rollover_address_merkle_tree_instructions,
        create_rollover_state_merkle_tree_instructions, register_test_forester,
        update_test_forester,
    },
    rpc::{errors::assert_rpc_error, rpc_connection::RpcConnection, SolanaRpcConnection},
    test_env::{
        get_test_env_accounts, register_program_with_registry_program,
        setup_test_programs_with_accounts,
    },
};
use solana_sdk::{
    instruction::Instruction,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
};

#[tokio::test]
async fn test_register_program() {
    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let random_program_keypair = Keypair::new();
    register_program_with_registry_program(
        &mut rpc,
        &env.governance_authority,
        &env.group_pda,
        &random_program_keypair,
    )
    .await
    .unwrap();
}

/// Test:
/// 1. SUCCESS: Register a forester
/// 2. SUCCESS: Update forester authority
/// 3. SUCESS: Register forester for epoch
#[tokio::test]
async fn test_register_and_update_forester_pda() {
    // TODO: add setup test programs wrapper that allows for non default protocol config
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
    println!("rpc.air -----------------------------------------");
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
    println!("registered _test_forester -----------------------------------------");

    // // 2. SUCCESS: Update forester authority
    // let new_forester_keypair = Keypair::new();
    // rpc.airdrop_lamports(&new_forester_keypair.pubkey(), 1_000_000_000)
    //     .await
    //     .unwrap();
    // let config = ForesterConfig { fee: 2 };

    // update_test_forester(
    //     &mut rpc,
    //     &forester_keypair,
    //     Some(&new_forester_keypair),
    //     config,
    // )
    // .await
    // .unwrap();
    let protocol_config = rpc
        .get_anchor_account::<ProtocolConfigPda>(&env.governance_authority_pda)
        .await
        .unwrap()
        .unwrap()
        .config;

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
    assert!(forester_epoch_pda.total_epoch_state_weight.is_none());
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
            .fetch_account_and_add_trees_with_schedule(&mut rpc, tree_accounts)
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
    // TODO: write an e2e test with multiple foresters - essentially integrate into e2e tests and make every round a slot
    // 1. create multiple foresters
    // 2. register them etc.
    // 3. iterate over every light slot
    // 3.1. give a random number of work items
    // 3.2. check for every forester who is eligible

    // create work 1 item in address and nullifier queue each
    let (mut state_merkle_tree_bundle, mut address_merkle_tree, mut rpc) = {
        let mut e2e_env = init_program_test_env(rpc, &env).await;
        e2e_env.create_address(None).await;
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
    println!("performed transactions -----------------------------------------");
    // perform 1 work
    nullify_compressed_accounts(
        &mut rpc,
        &forester_keypair,
        &mut state_merkle_tree_bundle,
        epoch,
    )
    .await;
    empty_address_queue_test(
        &forester_keypair,
        &mut rpc,
        &mut address_merkle_tree,
        false,
        epoch,
    )
    .await
    .unwrap();

    // advance epoch to report work and next registration phase
    rpc.warp_to_slot(
        registered_epoch.phases.report_work.start - protocol_config.registration_phase_length,
    )
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
    // TODO: check that we can still forest the last epoch
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

    // TODO: test claim once implemented
    // advance to post epoch phase
}
// TODO: test edge cases first and last slot of every phase
// TODO: add failing tests, perform actions in invalid phases, pass unrelated epoch and forester pda

/// Test:
/// 1. FAIL: Register a forester with invalid authority
/// 2. FAIL: Update forester authority with invalid authority
/// 3. FAIL: Nullify with invalid authority
/// 4. FAIL: Update address tree with invalid authority
/// 5. FAIL: Rollover address tree with invalid authority
/// 6. FAIL: Rollover state tree with invalid authority
#[tokio::test]
async fn failing_test_forester() {
    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let payer = rpc.get_payer().insecure_clone();
    // 1. FAIL: Register a forester with invalid authority
    {
        let result = register_test_forester(
            &mut rpc,
            &payer,
            &Keypair::new().pubkey(),
            ForesterConfig::default(),
        )
        .await;
        let expected_error_code = anchor_lang::error::ErrorCode::ConstraintAddress as u32;
        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 2. FAIL: Update forester authority with invalid authority
    {
        let forester_epoch_pda = get_forester_epoch_pda_address(&env.forester.pubkey(), 0).0;
        let instruction_data = light_registry::instruction::UpdateForesterEpochPda {
            authority: Keypair::new().pubkey(),
        };
        let accounts = light_registry::accounts::UpdateForesterEpochPda {
            forester_epoch_pda,
            signer: payer.pubkey(),
        };
        let ix = Instruction {
            program_id: light_registry::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
            .await;
        let expected_error_code = anchor_lang::error::ErrorCode::ConstraintAddress as u32;
        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 3. FAIL: Nullify with invalid authority
    {
        let expected_error_code =
            light_registry::errors::RegistryError::InvalidForester as u32 + 6000;
        let inputs = CreateNullifyInstructionInputs {
            authority: payer.pubkey(),
            nullifier_queue: env.nullifier_queue_pubkey,
            merkle_tree: env.merkle_tree_pubkey,
            change_log_indices: vec![1],
            leaves_queue_indices: vec![1u16],
            indices: vec![0u64],
            proofs: vec![vec![[0u8; 32]; 26]],
            derivation: payer.pubkey(),
        };
        let mut ix = create_nullify_instruction(inputs, 0);
        // Swap the derived forester pda with an initialized but invalid one.
        ix.accounts[0].pubkey = get_forester_epoch_pda_address(&env.forester.pubkey(), 0).0;
        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
            .await;
        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 4 FAIL: update address Merkle tree failed
    {
        let expected_error_code =
            light_registry::errors::RegistryError::InvalidForester as u32 + 6000;
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
            },
            0,
        );
        // Swap the derived forester pda with an initialized but invalid one.
        instruction.accounts[0].pubkey =
            get_forester_epoch_pda_address(&env.forester.pubkey(), 0).0;

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
        )
        .await;
        // Swap the derived forester pda with an initialized but invalid one.
        instructions[2].accounts[0].pubkey =
            get_forester_epoch_pda_address(&env.forester.pubkey(), 0).0;

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
            &new_nullifier_queue_keypair,
            &new_state_merkle_tree_keypair,
            &env.merkle_tree_pubkey,
            &env.nullifier_queue_pubkey,
            &new_cpi_context.pubkey(),
            0, // TODO: adapt epoch
        )
        .await;
        // Swap the derived forester pda with an initialized but invalid one.
        instructions[2].accounts[0].pubkey =
            get_forester_epoch_pda_address(&env.forester.pubkey(), 0).0;

        let result = rpc
            .create_and_send_transaction(
                &instructions,
                &authority.pubkey(),
                &[
                    &authority,
                    &new_nullifier_queue_keypair,
                    &new_state_merkle_tree_keypair,
                ],
            )
            .await;
        assert_rpc_error(result, 2, expected_error_code).unwrap();
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
        .get_anchor_account::<ForesterAccount>(&env_accounts.registered_forester_pda)
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
        Some(&updated_keypair),
        ForesterConfig::default(),
    )
    .await
    .unwrap();
    let forester_epoch = rpc
        .get_anchor_account::<ForesterAccount>(&env_accounts.registered_forester_pda)
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
    let (_, bump) = get_protocol_config_pda_address();
    let instruction = light_registry::instruction::UpdateGovernanceAuthority {
        new_authority: updated_keypair.pubkey(),
        bump,
    };
    let accounts = light_registry::accounts::UpdateAuthority {
        authority_pda: env_accounts.governance_authority_pda,
        authority: env_accounts.governance_authority.pubkey(),
        new_authority: updated_keypair.pubkey(),
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
    let authority_keypair =
        read_keypair_file("../../target/governance-authority-keypair.json").unwrap();
    let forester_keypair = read_keypair_file("../../target/forester-keypair.json").unwrap();
    println!("authority pubkey: {:?}", authority_keypair.pubkey());
    println!("forester pubkey: {:?}", forester_keypair.pubkey());
    setup_accounts_devnet(&authority_keypair, &forester_keypair).await;
}
