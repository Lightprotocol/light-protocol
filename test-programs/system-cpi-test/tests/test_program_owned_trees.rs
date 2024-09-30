#![cfg(feature = "test-sbf")]

use account_compression::sdk::create_insert_leaves_instruction;
use account_compression::utils::constants::{CPI_AUTHORITY_PDA_SEED, STATE_NULLIFIER_QUEUE_VALUES};
use account_compression::{
    AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig, QueueAccount,
    StateMerkleTreeAccount, StateMerkleTreeConfig,
};
use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use light_compressed_token::mint_sdk::create_mint_to_instruction;
use light_hasher::Poseidon;
use light_registry::account_compression_cpi::sdk::{
    create_nullify_instruction, get_registered_program_pda, CreateNullifyInstructionInputs,
};
use light_registry::protocol_config::state::ProtocolConfig;
use light_registry::utils::{
    get_cpi_authority_pda, get_forester_epoch_pda_from_authority, get_protocol_config_pda_address,
};
use light_test_utils::rpc::test_rpc::ProgramTestRpcConnection;
use light_test_utils::spl::create_mint_helper;
use light_test_utils::test_env::{
    initialize_new_group, register_program_with_registry_program, NOOP_PROGRAM_ID,
};
use light_test_utils::{
    airdrop_lamports, assert_rpc_error, create_account_instruction, get_concurrent_merkle_tree,
    FeeConfig, RpcConnection, RpcError, TransactionParams,
};
use light_test_utils::{
    assert_custom_error_or_program_error, indexer::TestIndexer,
    test_env::setup_test_programs_with_accounts,
};
use solana_sdk::instruction::Instruction;
use solana_sdk::signature::Signature;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};
use system_cpi_test::sdk::{
    create_initialize_address_merkle_tree_and_queue_instruction,
    create_initialize_merkle_tree_instruction,
};

#[tokio::test]
async fn test_program_owned_merkle_tree() {
    let (rpc, env) = setup_test_programs_with_accounts(Some(vec![(
        String::from("system_cpi_test"),
        system_cpi_test::ID,
    )]))
    .await;
    let payer = rpc.get_payer().await;
    let payer_pubkey = payer.pubkey();

    let program_owned_merkle_tree_keypair = Keypair::new();
    let program_owned_merkle_tree_pubkey = program_owned_merkle_tree_keypair.pubkey();
    let program_owned_nullifier_queue_keypair = Keypair::new();
    let cpi_context_keypair = Keypair::new();

    let test_indexer =
        TestIndexer::<ProgramTestRpcConnection>::init_from_env(&payer, &env, true, true).await;
    test_indexer
        .add_state_merkle_tree(
            &rpc,
            &program_owned_merkle_tree_keypair,
            &program_owned_nullifier_queue_keypair,
            &cpi_context_keypair,
            Some(light_compressed_token::ID),
            None,
        )
        .await;

    let recipient_keypair = Keypair::new();
    let mint = create_mint_helper(&rpc, &payer).await;
    let amount = 10000u64;
    let instruction = create_mint_to_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &mint,
        &program_owned_merkle_tree_pubkey,
        vec![amount; 1],
        vec![recipient_keypair.pubkey(); 1],
        None,
    );
    let pre_merkle_tree = get_concurrent_merkle_tree::<
        StateMerkleTreeAccount,
        ProgramTestRpcConnection,
        Poseidon,
        26,
    >(&rpc, program_owned_merkle_tree_pubkey)
    .await;
    let event = rpc
        .create_and_send_transaction_with_event(
            &[instruction],
            &payer_pubkey,
            &[&payer],
            Some(TransactionParams {
                num_new_addresses: 0,
                num_input_compressed_accounts: 0,
                num_output_compressed_accounts: 1,
                compress: 0,
                fee_config: FeeConfig::default(),
            }),
        )
        .await
        .unwrap()
        .unwrap();
    let post_merkle_tree = get_concurrent_merkle_tree::<
        StateMerkleTreeAccount,
        ProgramTestRpcConnection,
        Poseidon,
        26,
    >(&rpc, program_owned_merkle_tree_pubkey)
    .await;
    test_indexer
        .add_compressed_accounts_with_token_data(&event.0)
        .await;

    assert_ne!(post_merkle_tree.root(), pre_merkle_tree.root());

    {
        let state_merkle_trees = test_indexer.state.state_merkle_trees.read().await;
        assert_eq!(
            post_merkle_tree.root(),
            state_merkle_trees[1].merkle_tree.root()
        );
    }

    let invalid_program_owned_merkle_tree_keypair = Keypair::new();
    let invalid_program_owned_merkle_tree_pubkey =
        invalid_program_owned_merkle_tree_keypair.pubkey();
    let invalid_program_owned_nullifier_queue_keypair = Keypair::new();
    let cpi_context_keypair = Keypair::new();
    test_indexer
        .add_state_merkle_tree(
            &rpc,
            &invalid_program_owned_merkle_tree_keypair,
            &invalid_program_owned_nullifier_queue_keypair,
            &cpi_context_keypair,
            Some(Keypair::new().pubkey()),
            None,
        )
        .await;
    let recipient_keypair = Keypair::new();
    let instruction = create_mint_to_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &mint,
        &invalid_program_owned_merkle_tree_pubkey,
        vec![amount + 1; 1],
        vec![recipient_keypair.pubkey(); 1],
        None,
    );

    let latest_blockhash = rpc.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        latest_blockhash,
    );
    let res = rpc.process_transaction(transaction).await;
    assert_custom_error_or_program_error(
        res,
        light_system_program::errors::SystemProgramError::InvalidMerkleTreeOwner.into(),
    )
    .unwrap();
}

const CPI_SYSTEM_TEST_PROGRAM_ID_KEYPAIR: [u8; 64] = [
    57, 80, 188, 3, 162, 80, 232, 181, 222, 192, 247, 98, 140, 227, 70, 15, 169, 202, 73, 184, 23,
    90, 69, 95, 211, 74, 128, 232, 155, 216, 5, 230, 213, 158, 155, 203, 26, 211, 193, 195, 11,
    219, 9, 155, 58, 172, 58, 200, 254, 75, 231, 106, 31, 168, 183, 76, 179, 113, 234, 101, 191,
    99, 156, 98,
];

/// Test:
/// - Register the test program
/// - failing test registered program signer check
/// 1. FAIL: try to append leaves to the merkle tree from test program with invalid registered program account
/// 2. try to append leaves to the merkle tree from account compression program
/// - register the test program to the correct group
/// 3. SUCCEED: append leaves to the merkle tree from test program
/// - register the token program to the correct group
/// 4. FAIL: try to append leaves to the merkle tree from test program with invalid registered program account
/// 5. FAIL: rollover state Merkle tree  with invalid group
/// 6. FAIL: rollover address Merkle tree with invalid group
/// 7. FAIL: update address Merkle tree with invalid group
/// 8. FAIL: nullify leaves with invalid group
/// 9. FAIL: insert into address queue with invalid group
/// 10. FAIL: insert into nullifier queue with invalid group
/// 11. FAIL: create address Merkle tree with invalid group
/// 12. FAIL: create state Merkle tree with invalid group
#[tokio::test]
async fn test_invalid_registered_program() {
    let (mut rpc, env) = setup_test_programs_with_accounts(Some(vec![(
        String::from("system_cpi_test"),
        system_cpi_test::ID,
    )]))
    .await;
    let payer = env.forester.insecure_clone();
    airdrop_lamports(&mut rpc, &payer.pubkey(), 100_000_000_000)
        .await
        .unwrap();
    let group_seed_keypair = Keypair::new();
    let program_id_keypair = Keypair::from_bytes(&CPI_SYSTEM_TEST_PROGRAM_ID_KEYPAIR).unwrap();
    println!("program_id_keypair: {:?}", program_id_keypair.pubkey());
    let invalid_group_pda =
        initialize_new_group(&group_seed_keypair, &payer, &mut rpc, payer.pubkey()).await;
    let invalid_group_registered_program_pda =
        register_program(&mut rpc, &payer, &program_id_keypair, &invalid_group_pda)
            .await
            .unwrap();
    let invalid_group_state_merkle_tree = Keypair::new();
    let invalid_group_nullifier_queue = Keypair::new();
    create_state_merkle_tree_and_queue_account(
        &payer,
        &mut rpc,
        &invalid_group_state_merkle_tree,
        &invalid_group_nullifier_queue,
        None,
        3,
        &StateMerkleTreeConfig::default(),
        &NullifierQueueConfig::default(),
        false,
    )
    .await
    .unwrap();
    let invalid_group_address_merkle_tree = Keypair::new();
    let invalid_group_address_queue = Keypair::new();
    create_address_merkle_tree_and_queue_account(
        &payer,
        &mut rpc,
        &invalid_group_address_merkle_tree,
        &invalid_group_address_queue,
        None,
        &AddressMerkleTreeConfig::default(),
        &AddressQueueConfig::default(),
        3,
        false,
    )
    .await
    .unwrap();

    let merkle_tree_pubkey = env.merkle_tree_pubkey;

    // invoke account compression program through system cpi test
    // 1. the program is registered with a different group than the Merkle tree
    {
        let derived_address =
            Pubkey::find_program_address(&[CPI_AUTHORITY_PDA_SEED], &system_cpi_test::ID).0;
        let accounts = system_cpi_test::accounts::AppendLeavesAccountCompressionProgram {
            signer: payer.pubkey(),
            registered_program_pda: invalid_group_registered_program_pda,
            noop_program: Pubkey::new_from_array(
                account_compression::utils::constants::NOOP_PUBKEY,
            ),
            account_compression_program: account_compression::ID,
            cpi_signer: derived_address,
            system_program: system_program::ID,
            merkle_tree: merkle_tree_pubkey,
            queue: merkle_tree_pubkey, // not used in this ix
        };

        let instruction_data =
            system_cpi_test::instruction::AppendLeavesAccountCompressionProgram {};
        let instruction = Instruction {
            program_id: system_cpi_test::ID,
            accounts: [accounts.to_account_metas(Some(true))].concat(),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await;
        let expected_error_code =
            account_compression::errors::AccountCompressionErrorCode::InvalidAuthority.into();
        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }

    // 2. directly invoke account compression program
    {
        let instruction = create_insert_leaves_instruction(
            vec![(0, [1u8; 32])],
            payer.pubkey(),
            payer.pubkey(),
            vec![merkle_tree_pubkey],
        );
        let expected_error_code =
            account_compression::errors::AccountCompressionErrorCode::InvalidAuthority.into();

        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await;
        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    let other_program_id_keypair = Keypair::new();
    let token_program_registered_program_pda = register_program_with_registry_program(
        &mut rpc,
        &env.governance_authority,
        &env.group_pda,
        &other_program_id_keypair,
    )
    .await
    .unwrap();
    // 4. use registered_program_pda of other program
    {
        let derived_address =
            Pubkey::find_program_address(&[CPI_AUTHORITY_PDA_SEED], &system_cpi_test::ID).0;
        let accounts = system_cpi_test::accounts::AppendLeavesAccountCompressionProgram {
            signer: payer.pubkey(),
            registered_program_pda: token_program_registered_program_pda,
            noop_program: Pubkey::new_from_array(
                account_compression::utils::constants::NOOP_PUBKEY,
            ),
            account_compression_program: account_compression::ID,
            cpi_signer: derived_address,
            system_program: system_program::ID,
            merkle_tree: merkle_tree_pubkey,
            queue: merkle_tree_pubkey, // not used in this ix
        };

        let instruction_data =
            system_cpi_test::instruction::AppendLeavesAccountCompressionProgram {};
        let instruction = Instruction {
            program_id: system_cpi_test::ID,
            accounts: [accounts.to_account_metas(Some(true))].concat(),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await;
        let expected_error_code =
            account_compression::errors::AccountCompressionErrorCode::InvalidAuthority.into();

        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 6. rollover state Merkle tree with invalid group
    {
        let new_merkle_tree_keypair = Keypair::new();
        let new_queue_keypair = Keypair::new();
        let new_cpi_context_keypair = Keypair::new();
        let (cpi_authority, bump) = get_cpi_authority_pda();
        let registered_program_pda = get_registered_program_pda(&light_registry::ID);
        let registered_forester_pda =
            get_forester_epoch_pda_from_authority(&env.forester.pubkey(), 0).0;
        let protocol_config_pda = get_protocol_config_pda_address().0;

        let instruction_data =
            light_registry::instruction::RolloverStateMerkleTreeAndQueue { bump };
        let accounts = light_registry::accounts::RolloverStateMerkleTreeAndQueue {
            account_compression_program: account_compression::ID,
            registered_forester_pda: Some(registered_forester_pda),
            cpi_authority,
            authority: payer.pubkey(),
            registered_program_pda,
            new_merkle_tree: new_merkle_tree_keypair.pubkey(),
            new_queue: new_queue_keypair.pubkey(),
            old_merkle_tree: invalid_group_state_merkle_tree.pubkey(),
            old_queue: invalid_group_nullifier_queue.pubkey(),
            cpi_context_account: new_cpi_context_keypair.pubkey(),
            light_system_program: light_system_program::ID,
            protocol_config_pda,
        };
        let size = QueueAccount::size(STATE_NULLIFIER_QUEUE_VALUES as usize).unwrap();
        let create_nullifier_queue_instruction = create_account_instruction(
            &payer.pubkey(),
            size,
            rpc.get_minimum_balance_for_rent_exemption(size)
                .await
                .unwrap(),
            &account_compression::ID,
            Some(&new_queue_keypair),
        );
        let size = StateMerkleTreeAccount::size(
            account_compression::utils::constants::STATE_MERKLE_TREE_HEIGHT as usize,
            account_compression::utils::constants::STATE_MERKLE_TREE_CHANGELOG as usize,
            account_compression::utils::constants::STATE_MERKLE_TREE_ROOTS as usize,
            account_compression::utils::constants::STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
        );
        let create_state_merkle_tree_instruction = create_account_instruction(
            &payer.pubkey(),
            size,
            rpc.get_minimum_balance_for_rent_exemption(size)
                .await
                .unwrap(),
            &account_compression::ID,
            Some(&new_merkle_tree_keypair),
        );
        let size = ProtocolConfig::default().cpi_context_size as usize;
        let create_cpi_context_account_instruction = create_account_instruction(
            &payer.pubkey(),
            size,
            rpc.get_minimum_balance_for_rent_exemption(size)
                .await
                .unwrap(),
            &light_system_program::ID,
            Some(&new_cpi_context_keypair),
        );
        let instruction = Instruction {
            program_id: light_registry::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(
                &[
                    create_nullifier_queue_instruction,
                    create_state_merkle_tree_instruction,
                    create_cpi_context_account_instruction,
                    instruction,
                ],
                &payer.pubkey(),
                &[
                    &payer,
                    &new_merkle_tree_keypair,
                    &new_queue_keypair,
                    &new_cpi_context_keypair,
                ],
            )
            .await;
        let expected_error_code =
            account_compression::errors::AccountCompressionErrorCode::InvalidAuthority.into();

        assert_rpc_error(result, 3, expected_error_code).unwrap();
    }
    // 6. rollover address Merkle tree with invalid group
    {
        let new_merkle_tree_keypair = Keypair::new();
        let new_queue_keypair = Keypair::new();
        let (cpi_authority, bump) = get_cpi_authority_pda();
        let registered_program_pda = get_registered_program_pda(&light_registry::ID);
        let instruction_data =
            light_registry::instruction::RolloverAddressMerkleTreeAndQueue { bump };
        let registered_forester_pda =
            get_forester_epoch_pda_from_authority(&env.forester.pubkey(), 0).0;

        let accounts = light_registry::accounts::RolloverAddressMerkleTreeAndQueue {
            account_compression_program: account_compression::ID,
            registered_forester_pda: Some(registered_forester_pda),
            cpi_authority,
            authority: payer.pubkey(),
            registered_program_pda,
            new_merkle_tree: new_merkle_tree_keypair.pubkey(),
            new_queue: new_queue_keypair.pubkey(),
            old_merkle_tree: invalid_group_address_merkle_tree.pubkey(),
            old_queue: invalid_group_address_queue.pubkey(),
        };
        let size = QueueAccount::size(
            account_compression::utils::constants::ADDRESS_QUEUE_VALUES as usize,
        )
        .unwrap();
        let create_nullifier_queue_instruction = create_account_instruction(
            &payer.pubkey(),
            size,
            rpc.get_minimum_balance_for_rent_exemption(size)
                .await
                .unwrap(),
            &account_compression::ID,
            Some(&new_queue_keypair),
        );
        let size = account_compression::state::AddressMerkleTreeAccount::size(
            account_compression::utils::constants::ADDRESS_MERKLE_TREE_HEIGHT as usize,
            account_compression::utils::constants::ADDRESS_MERKLE_TREE_CHANGELOG as usize,
            account_compression::utils::constants::ADDRESS_MERKLE_TREE_ROOTS as usize,
            account_compression::utils::constants::ADDRESS_MERKLE_TREE_CANOPY_DEPTH as usize,
            account_compression::utils::constants::ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG as usize,
        );
        let create_state_merkle_tree_instruction = create_account_instruction(
            &payer.pubkey(),
            size,
            rpc.get_minimum_balance_for_rent_exemption(size)
                .await
                .unwrap(),
            &account_compression::ID,
            Some(&new_merkle_tree_keypair),
        );
        let instruction = Instruction {
            program_id: light_registry::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(
                &[
                    create_nullifier_queue_instruction,
                    create_state_merkle_tree_instruction,
                    instruction,
                ],
                &payer.pubkey(),
                &[&payer, &new_merkle_tree_keypair, &new_queue_keypair],
            )
            .await;
        let expected_error_code =
            account_compression::errors::AccountCompressionErrorCode::InvalidAuthority.into();

        assert_rpc_error(result, 2, expected_error_code).unwrap();
    }
    // 7. nullify with invalid group
    {
        let inputs = CreateNullifyInstructionInputs {
            authority: payer.pubkey(),
            nullifier_queue: invalid_group_nullifier_queue.pubkey(),
            merkle_tree: invalid_group_state_merkle_tree.pubkey(),
            change_log_indices: vec![1],
            leaves_queue_indices: vec![1u16],
            indices: vec![0u64],
            proofs: vec![vec![[0u8; 32]; 26]],
            derivation: env.forester.pubkey(),
            is_metadata_forester: false,
        };
        let ix = create_nullify_instruction(inputs, 0);

        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
            .await;
        let expected_error_code =
            account_compression::errors::AccountCompressionErrorCode::InvalidAuthority.into();

        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 8. update address with invalid group
    {
        let register_program_pda = get_registered_program_pda(&light_registry::ID);
        let registered_forester_pda =
            get_forester_epoch_pda_from_authority(&env.forester.pubkey(), 0).0;
        let (cpi_authority, bump) = get_cpi_authority_pda();
        let instruction_data = light_registry::instruction::UpdateAddressMerkleTree {
            bump,
            changelog_index: 1,
            indexed_changelog_index: 1,
            value: 1u16,
            low_address_index: 1,
            low_address_proof: [[0u8; 32]; 16],
            low_address_next_index: 1,
            low_address_next_value: [0u8; 32],
            low_address_value: [0u8; 32],
        };
        let accounts = light_registry::accounts::UpdateAddressMerkleTree {
            authority: payer.pubkey(),
            registered_forester_pda: Some(registered_forester_pda),
            registered_program_pda: register_program_pda,
            queue: invalid_group_address_queue.pubkey(),
            merkle_tree: invalid_group_address_merkle_tree.pubkey(),
            log_wrapper: NOOP_PROGRAM_ID,
            cpi_authority,
            account_compression_program: account_compression::ID,
        };
        let ix = Instruction {
            program_id: light_registry::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
            .await;
        let expected_error_code =
            account_compression::errors::AccountCompressionErrorCode::InvalidAuthority.into();

        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 9. insert into address queue with invalid group
    {
        let derived_address =
            Pubkey::find_program_address(&[CPI_AUTHORITY_PDA_SEED], &system_cpi_test::ID).0;
        let accounts = system_cpi_test::accounts::AppendLeavesAccountCompressionProgram {
            signer: payer.pubkey(),
            registered_program_pda: token_program_registered_program_pda,
            noop_program: Pubkey::new_from_array(
                account_compression::utils::constants::NOOP_PUBKEY,
            ),
            account_compression_program: account_compression::ID,
            cpi_signer: derived_address,
            system_program: system_program::ID,
            merkle_tree: env.address_merkle_tree_pubkey,
            queue: env.address_merkle_tree_queue_pubkey,
        };

        let instruction_data = system_cpi_test::instruction::InsertIntoAddressQueue {};
        let instruction = Instruction {
            program_id: system_cpi_test::ID,
            accounts: [accounts.to_account_metas(Some(true))].concat(),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await;
        let expected_error_code =
            account_compression::errors::AccountCompressionErrorCode::InvalidAuthority.into();

        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 10. insert into nullifier queue with invalid group
    {
        let derived_address =
            Pubkey::find_program_address(&[CPI_AUTHORITY_PDA_SEED], &system_cpi_test::ID).0;
        let accounts = system_cpi_test::accounts::AppendLeavesAccountCompressionProgram {
            signer: payer.pubkey(),
            registered_program_pda: token_program_registered_program_pda,
            noop_program: Pubkey::new_from_array(
                account_compression::utils::constants::NOOP_PUBKEY,
            ),
            account_compression_program: account_compression::ID,
            cpi_signer: derived_address,
            system_program: system_program::ID,
            merkle_tree: env.merkle_tree_pubkey,
            queue: env.nullifier_queue_pubkey,
        };

        let instruction_data = system_cpi_test::instruction::InsertIntoNullifierQueue {};
        let instruction = Instruction {
            program_id: system_cpi_test::ID,
            accounts: [accounts.to_account_metas(Some(true))].concat(),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await;
        let expected_error_code =
            account_compression::errors::AccountCompressionErrorCode::InvalidAuthority.into();

        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }

    // 11. create address Merkle tree with invalid group
    {
        let invalid_group_state_merkle_tree = Keypair::new();
        let invalid_group_nullifier_queue = Keypair::new();
        let result = create_state_merkle_tree_and_queue_account(
            &payer,
            &mut rpc,
            &invalid_group_state_merkle_tree,
            &invalid_group_nullifier_queue,
            None,
            3,
            &StateMerkleTreeConfig::default(),
            &NullifierQueueConfig::default(),
            true,
        )
        .await;
        let expected_error_code =
            account_compression::errors::AccountCompressionErrorCode::InvalidAuthority.into();

        assert_rpc_error(result, 2, expected_error_code).unwrap();
    }
    {
        let invalid_group_address_merkle_tree = Keypair::new();
        let invalid_group_address_queue = Keypair::new();
        let result = create_address_merkle_tree_and_queue_account(
            &payer,
            &mut rpc,
            &invalid_group_address_merkle_tree,
            &invalid_group_address_queue,
            None,
            &AddressMerkleTreeConfig::default(),
            &AddressQueueConfig::default(),
            3,
            true,
        )
        .await;
        let expected_error_code =
            account_compression::errors::AccountCompressionErrorCode::InvalidAuthority.into();
        assert_rpc_error(result, 2, expected_error_code).unwrap();
    }
}

pub async fn register_program(
    rpc: &mut ProgramTestRpcConnection,
    authority: &Keypair,
    program_id_keypair: &Keypair,
    group_account: &Pubkey,
) -> Result<Pubkey, RpcError> {
    let registered_program_pda = Pubkey::find_program_address(
        &[program_id_keypair.pubkey().to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0;

    let accounts = account_compression::accounts::RegisterProgramToGroup {
        authority: authority.pubkey(),
        program_to_be_registered: program_id_keypair.pubkey(),
        system_program: system_program::ID,
        registered_program_pda,
        group_authority_pda: *group_account,
    };
    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: account_compression::instruction::RegisterProgramToGroup {}.data(),
    };

    rpc.create_and_send_transaction(
        &[instruction],
        &authority.pubkey(),
        &[authority, program_id_keypair],
    )
    .await?;

    Ok(registered_program_pda)
}

#[allow(clippy::too_many_arguments)]
pub async fn create_state_merkle_tree_and_queue_account<R: RpcConnection>(
    payer: &Keypair,
    rpc: &mut R,
    merkle_tree_keypair: &Keypair,
    nullifier_queue_keypair: &Keypair,
    program_owner: Option<Pubkey>,
    index: u64,
    merkle_tree_config: &StateMerkleTreeConfig,
    queue_config: &NullifierQueueConfig,
    invalid_group: bool,
) -> Result<Signature, RpcError> {
    let size = StateMerkleTreeAccount::size(
        merkle_tree_config.height as usize,
        merkle_tree_config.changelog_size as usize,
        merkle_tree_config.roots_size as usize,
        merkle_tree_config.canopy_depth as usize,
    );

    let merkle_tree_account_create_ix = create_account_instruction(
        &payer.pubkey(),
        size,
        rpc.get_minimum_balance_for_rent_exemption(size).await?,
        &account_compression::ID,
        Some(merkle_tree_keypair),
    );
    let size = QueueAccount::size(queue_config.capacity as usize).unwrap();
    let nullifier_queue_account_create_ix = create_account_instruction(
        &payer.pubkey(),
        size,
        rpc.get_minimum_balance_for_rent_exemption(size).await?,
        &account_compression::ID,
        Some(nullifier_queue_keypair),
    );

    let instruction = create_initialize_merkle_tree_instruction(
        payer.pubkey(),
        merkle_tree_keypair.pubkey(),
        nullifier_queue_keypair.pubkey(),
        merkle_tree_config.clone(),
        queue_config.clone(),
        program_owner,
        index,
        0, // TODO: replace with CPI_CONTEXT_ACCOUNT_RENT
        invalid_group,
    );

    let transaction = Transaction::new_signed_with_payer(
        &[
            merkle_tree_account_create_ix,
            nullifier_queue_account_create_ix,
            instruction,
        ],
        Some(&payer.pubkey()),
        &vec![payer, merkle_tree_keypair, nullifier_queue_keypair],
        rpc.get_latest_blockhash().await?,
    );
    rpc.process_transaction(transaction.clone()).await
}

#[allow(clippy::too_many_arguments)]
#[inline(never)]
pub async fn create_address_merkle_tree_and_queue_account<R: RpcConnection>(
    payer: &Keypair,
    context: &mut R,
    address_merkle_tree_keypair: &Keypair,
    address_queue_keypair: &Keypair,
    program_owner: Option<Pubkey>,
    merkle_tree_config: &AddressMerkleTreeConfig,
    queue_config: &AddressQueueConfig,
    index: u64,
    invalid_group: bool,
) -> Result<Signature, RpcError> {
    let size = QueueAccount::size(queue_config.capacity as usize).unwrap();
    let account_create_ix = create_account_instruction(
        &payer.pubkey(),
        size,
        context.get_minimum_balance_for_rent_exemption(size).await?,
        &account_compression::ID,
        Some(address_queue_keypair),
    );

    let size = account_compression::state::AddressMerkleTreeAccount::size(
        merkle_tree_config.height as usize,
        merkle_tree_config.changelog_size as usize,
        merkle_tree_config.roots_size as usize,
        merkle_tree_config.canopy_depth as usize,
        merkle_tree_config.address_changelog_size as usize,
    );
    let mt_account_create_ix = create_account_instruction(
        &payer.pubkey(),
        size,
        context.get_minimum_balance_for_rent_exemption(size).await?,
        &account_compression::ID,
        Some(address_merkle_tree_keypair),
    );
    let instruction = create_initialize_address_merkle_tree_and_queue_instruction(
        index,
        payer.pubkey(),
        program_owner,
        address_merkle_tree_keypair.pubkey(),
        address_queue_keypair.pubkey(),
        merkle_tree_config.clone(),
        queue_config.clone(),
        invalid_group,
    );
    let transaction = Transaction::new_signed_with_payer(
        &[account_create_ix, mt_account_create_ix, instruction],
        Some(&payer.pubkey()),
        &vec![&payer, &address_queue_keypair, &address_merkle_tree_keypair],
        context.get_latest_blockhash().await?,
    );
    context.process_transaction(transaction.clone()).await
}
