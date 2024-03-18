#![cfg(feature = "test-sbf")]

use std::str::FromStr;

use account_compression::{
    self, from_vec,
    indexed_array_sdk::create_initialize_indexed_array_instruction,
    instructions::append_leaves::sdk::{
        create_initialize_merkle_tree_instruction, create_insert_leaves_instruction,
    },
    utils::constants::GROUP_AUTHORITY_SEED,
    GroupAuthority, StateMerkleTreeAccount, ID,
};
use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use light_concurrent_merkle_tree::{ConcurrentMerkleTree, ConcurrentMerkleTree26};
use light_hasher::{zero_bytes::poseidon::ZERO_BYTES, Poseidon};
use light_test_utils::{
    airdrop_lamports, create_account_instruction, create_and_send_transaction, get_account,
    AccountZeroCopy,
};
use solana_program_test::ProgramTest;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

#[tokio::test]
async fn test_create_and_update_group() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    let compressed_pda_id =
        Pubkey::from_str("6UqiSPd2mRCTTwkzhcs1M6DGYsqHWd5jiPueX3LwDMXQ").unwrap();
    program_test.add_program("psp_compressed_pda", compressed_pda_id, None);

    program_test.set_compute_max_units(1_400_000u64);

    let mut context = program_test.start_with_context().await;

    let seed = [1u8; 32];
    let group_accounts = anchor_lang::prelude::Pubkey::find_program_address(
        &[GROUP_AUTHORITY_SEED, seed.as_slice()],
        &account_compression::ID,
    );

    let instruction_data = account_compression::instruction::InitializeGroupAuthority {
        _seed: seed,
        authority: context.payer.pubkey(),
    };

    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: vec![
            AccountMeta::new(context.payer.pubkey(), true),
            AccountMeta::new(group_accounts.0, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: instruction_data.data(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &vec![&context.payer],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let group_authority = get_account::<GroupAuthority>(&mut context, group_accounts.0).await;
    assert_eq!(group_authority.authority, context.payer.pubkey());
    assert_eq!(group_authority.seed, seed);

    let updated_keypair = Keypair::new();
    let update_group_authority_ix = account_compression::instruction::UpdateGroupAuthority {
        authority: updated_keypair.pubkey(),
    };

    // update with new authority
    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: vec![
            AccountMeta::new(context.payer.pubkey(), true),
            AccountMeta::new(group_accounts.0, false),
            AccountMeta::new_readonly(updated_keypair.pubkey(), false),
        ],
        data: update_group_authority_ix.data(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &vec![&context.payer],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let group_authority = get_account::<GroupAuthority>(&mut context, group_accounts.0).await;

    assert_eq!(group_authority.authority, updated_keypair.pubkey());
    assert_eq!(group_authority.seed, seed);

    // update with old authority should fail
    let update_group_authority_ix = account_compression::instruction::UpdateGroupAuthority {
        authority: context.payer.pubkey(),
    };
    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: vec![
            AccountMeta::new(context.payer.pubkey(), true),
            AccountMeta::new(group_accounts.0, false),
            AccountMeta::new_readonly(updated_keypair.pubkey(), false),
        ],
        data: update_group_authority_ix.data(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &vec![&context.payer],
        context.last_blockhash,
    );
    let update_error = context.banks_client.process_transaction(transaction).await;
    assert!(update_error.is_err());

    airdrop_lamports(&mut context, &updated_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    // add new program to group
    let registered_program_pda =
        Pubkey::find_program_address(&[compressed_pda_id.to_bytes().as_slice()], &ID).0;

    let register_program_ix = account_compression::instruction::RegisterProgramToGroup {
        program_id: compressed_pda_id,
    };

    // update with new authority
    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: vec![
            AccountMeta::new(updated_keypair.pubkey(), true),
            AccountMeta::new(registered_program_pda, false),
            AccountMeta::new(group_accounts.0, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: register_program_ix.data(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&updated_keypair.pubkey()),
        &vec![&updated_keypair],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_init_and_insert_leaves_into_merkle_tree() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    program_test.add_program(
        "spl_noop",
        account_compression::state::change_log_event::NOOP_PROGRAM_ID,
        None,
    );

    program_test.set_compute_max_units(1_400_000u64);
    let mut context = program_test.start_with_context().await;

    let payer = context.payer.insecure_clone();
    let payer_pubkey = context.payer.pubkey();

    let merkle_tree_keypair = Keypair::new();
    let account_create_ix = create_account_instruction(
        &context.payer.pubkey(),
        StateMerkleTreeAccount::LEN,
        context
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(account_compression::StateMerkleTreeAccount::LEN),
        &ID,
        Some(&merkle_tree_keypair),
    );
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();

    let instruction =
        create_initialize_merkle_tree_instruction(context.payer.pubkey(), merkle_tree_pubkey);

    let transaction = Transaction::new_signed_with_payer(
        &[account_create_ix, instruction],
        Some(&context.payer.pubkey()),
        &vec![&context.payer, &merkle_tree_keypair],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction.clone())
        .await
        .unwrap();
    let merkle_tree = AccountZeroCopy::<account_compression::StateMerkleTreeAccount>::new(
        &mut context,
        merkle_tree_pubkey,
    )
    .await;
    assert_eq!(merkle_tree.deserialized().owner, payer_pubkey);
    assert_eq!(merkle_tree.deserialized().delegate, payer_pubkey);
    assert_eq!(merkle_tree.deserialized().index, 1);

    // insertions with merkle tree leaves missmatch should fail
    let instruction_data = account_compression::instruction::AppendLeavesToMerkleTrees {
        leaves: vec![[1u8; 32], [2u8; 32]],
    };

    let accounts = account_compression::accounts::AppendLeaves {
        authority: context.payer.pubkey(),
        registered_program_pda: None,
        log_wrapper: account_compression::state::change_log_event::NOOP_PROGRAM_ID,
    };

    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: vec![
            accounts.to_account_metas(Some(true)),
            vec![AccountMeta::new(merkle_tree_pubkey, false)],
        ]
        .concat(),
        data: instruction_data.data(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &vec![&context.payer],
        context.last_blockhash,
    );
    let remaining_accounts_missmatch_error =
        context.banks_client.process_transaction(transaction).await;
    assert!(remaining_accounts_missmatch_error.is_err());

    let old_merkle_tree = AccountZeroCopy::<account_compression::StateMerkleTreeAccount>::new(
        &mut context,
        merkle_tree_pubkey,
    )
    .await;
    let old_merkle_tree = old_merkle_tree.deserialized().copy_merkle_tree().unwrap();

    let instruction = [create_insert_leaves_instruction(
        vec![[1u8; 32], [2u8; 32]],
        context.payer.pubkey(),
        vec![merkle_tree_pubkey, merkle_tree_pubkey],
    )];

    create_and_send_transaction(&mut context, &instruction, &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    let merkle_tree = AccountZeroCopy::<account_compression::StateMerkleTreeAccount>::new(
        &mut context,
        merkle_tree_pubkey,
    )
    .await;
    let merkle_tree = merkle_tree.deserialized().copy_merkle_tree().unwrap();
    assert_eq!(merkle_tree.next_index, old_merkle_tree.next_index + 2);

    let mut reference_merkle_tree = ConcurrentMerkleTree26::<Poseidon>::new(
        account_compression::utils::constants::STATE_MERKLE_TREE_HEIGHT,
        account_compression::utils::constants::STATE_MERKLE_TREE_CHANGELOG,
        account_compression::utils::constants::STATE_MERKLE_TREE_ROOTS,
        account_compression::utils::constants::STATE_MERKLE_TREE_CANOPY_DEPTH,
    );
    reference_merkle_tree.init().unwrap();
    reference_merkle_tree
        .append_batch(&[&[1u8; 32], &[2u8; 32]])
        .unwrap();
    assert_eq!(
        merkle_tree.root().unwrap(),
        reference_merkle_tree.root().unwrap()
    );
}

#[tokio::test]
async fn test_init_and_insert_into_indexed_array() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    program_test.add_program(
        "spl_noop",
        account_compression::state::change_log_event::NOOP_PROGRAM_ID,
        None,
    );

    program_test.set_compute_max_units(1_400_000u64);
    let mut context = program_test.start_with_context().await;
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();

    let indexed_array_keypair = Keypair::new();

    let account_create_ix = create_account_instruction(
        &payer_pubkey,
        account_compression::IndexedArrayAccount::LEN,
        context
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(account_compression::IndexedArrayAccount::LEN),
        &ID,
        Some(&indexed_array_keypair),
    );
    let indexed_array_pubkey = indexed_array_keypair.pubkey();

    let instruction =
        create_initialize_indexed_array_instruction(payer_pubkey, indexed_array_pubkey, 1u64);

    let transaction = Transaction::new_signed_with_payer(
        &[account_create_ix, instruction],
        Some(&context.payer.pubkey()),
        &vec![&context.payer, &indexed_array_keypair],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction.clone())
        .await
        .unwrap();

    let array = AccountZeroCopy::<account_compression::IndexedArrayAccount>::new(
        &mut context,
        indexed_array_pubkey,
    )
    .await;
    assert_eq!(array.deserialized().owner, payer_pubkey);
    assert_eq!(array.deserialized().delegate, payer_pubkey);
    assert_eq!(array.deserialized().index, 1);
    let indexed_array = array.deserialized().indexed_array;
    assert_eq!(indexed_array[0].element, [0u8; 32]);
    assert_eq!(indexed_array[0].merkle_tree_overwrite_sequence_number, 0);

    // TODO: investigate why this fails with 0 0
    let instruction_data = account_compression::instruction::InsertIntoIndexedArrays {
        elements: vec![[1u8; 32], [2u8; 32]],
    };
    let accounts = account_compression::accounts::InsertIntoIndexedArrays {
        authority: context.payer.pubkey(),
        registered_program_pda: None,
    };
    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: vec![
            accounts.to_account_metas(Some(true)),
            vec![
                AccountMeta::new(indexed_array_pubkey, false),
                AccountMeta::new(indexed_array_pubkey, false),
            ],
        ]
        .concat(),
        data: instruction_data.data(),
    };
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &vec![&context.payer],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction.clone())
        .await
        .unwrap();
    let array = AccountZeroCopy::<account_compression::IndexedArrayAccount>::new(
        &mut context,
        indexed_array_pubkey,
    )
    .await;

    let indexed_array = array.deserialized().indexed_array;
    assert_eq!(indexed_array[0].element, [1u8; 32]);
    assert_eq!(indexed_array[1].element, [2u8; 32]);
    assert_eq!(indexed_array[0].merkle_tree_overwrite_sequence_number, 0);
    assert_eq!(indexed_array[1].merkle_tree_overwrite_sequence_number, 0);
}

#[tokio::test]
async fn test_nullify_leaves() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    program_test.add_program(
        "spl_noop",
        account_compression::state::change_log_event::NOOP_PROGRAM_ID,
        None,
    );

    program_test.set_compute_max_units(1_400_000u64);
    let mut context = program_test.start_with_context().await;

    let payer = context.payer.insecure_clone();
    let payer_pubkey = context.payer.pubkey();

    let merkle_tree_keypair = Keypair::new();
    let account_create_ix = create_account_instruction(
        &context.payer.pubkey(),
        StateMerkleTreeAccount::LEN,
        context
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(account_compression::StateMerkleTreeAccount::LEN),
        &ID,
        Some(&merkle_tree_keypair),
    );
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();

    let instruction =
        create_initialize_merkle_tree_instruction(context.payer.pubkey(), merkle_tree_pubkey);

    let transaction = Transaction::new_signed_with_payer(
        &[account_create_ix, instruction],
        Some(&context.payer.pubkey()),
        &vec![&context.payer, &merkle_tree_keypair],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction.clone())
        .await
        .unwrap();
    let merkle_tree = AccountZeroCopy::<account_compression::StateMerkleTreeAccount>::new(
        &mut context,
        merkle_tree_pubkey,
    )
    .await;
    assert_eq!(merkle_tree.deserialized().owner, payer_pubkey);
    assert_eq!(merkle_tree.deserialized().delegate, payer_pubkey);
    assert_eq!(merkle_tree.deserialized().index, 1);

    let indexed_array_keypair = Keypair::new();
    let account_create_ix = crate::create_account_instruction(
        &payer.pubkey(),
        account_compression::IndexedArrayAccount::LEN,
        context
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(account_compression::IndexedArrayAccount::LEN),
        &account_compression::ID,
        Some(&indexed_array_keypair),
    );
    let indexed_array_pubkey = indexed_array_keypair.pubkey();
    let instruction = create_initialize_indexed_array_instruction(
        payer.pubkey(),
        indexed_array_keypair.pubkey(),
        0,
    );
    let transaction = Transaction::new_signed_with_payer(
        &[account_create_ix, instruction],
        Some(&payer.pubkey()),
        &vec![&payer, &indexed_array_keypair],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction.clone())
        .await
        .unwrap();

    // insertions with merkle tree leaves missmatch should fail
    let instruction_data = account_compression::instruction::AppendLeavesToMerkleTrees {
        leaves: vec![[1u8; 32], [2u8; 32]],
    };

    let accounts = account_compression::accounts::AppendLeaves {
        authority: context.payer.pubkey(),
        registered_program_pda: None,
        log_wrapper: account_compression::state::change_log_event::NOOP_PROGRAM_ID,
    };

    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: vec![
            accounts.to_account_metas(Some(true)),
            vec![AccountMeta::new(merkle_tree_pubkey, false)],
        ]
        .concat(),
        data: instruction_data.data(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &vec![&context.payer],
        context.last_blockhash,
    );
    let remaining_accounts_missmatch_error =
        context.banks_client.process_transaction(transaction).await;
    assert!(remaining_accounts_missmatch_error.is_err());

    let old_merkle_tree = AccountZeroCopy::<account_compression::StateMerkleTreeAccount>::new(
        &mut context,
        merkle_tree_pubkey,
    )
    .await;
    let old_merkle_tree = old_merkle_tree.deserialized().copy_merkle_tree().unwrap();

    let instruction = [create_insert_leaves_instruction(
        vec![[1u8; 32], [2u8; 32]],
        context.payer.pubkey(),
        vec![merkle_tree_pubkey, merkle_tree_pubkey],
    )];

    create_and_send_transaction(&mut context, &instruction, &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    let merkle_tree = AccountZeroCopy::<account_compression::StateMerkleTreeAccount>::new(
        &mut context,
        merkle_tree_pubkey,
    )
    .await;
    let merkle_tree = merkle_tree.deserialized().copy_merkle_tree().unwrap();
    assert_eq!(merkle_tree.next_index, old_merkle_tree.next_index + 2);

    let mut reference_merkle_tree = light_merkle_tree_reference::MerkleTree::<Poseidon>::new(
        account_compression::utils::constants::STATE_MERKLE_TREE_HEIGHT,
        account_compression::utils::constants::STATE_MERKLE_TREE_ROOTS,
        account_compression::utils::constants::STATE_MERKLE_TREE_CANOPY_DEPTH,
    )
    .unwrap();
    reference_merkle_tree.append(&[1u8; 32]).unwrap();
    reference_merkle_tree.append(&[2u8; 32]).unwrap();
    assert_eq!(
        merkle_tree.root().unwrap(),
        reference_merkle_tree.root().unwrap()
    );

    // TODO: investigate why this fails with 0 0
    let instruction_data = account_compression::instruction::InsertIntoIndexedArrays {
        elements: vec![[1u8; 32], [2u8; 32]],
    };
    let accounts = account_compression::accounts::InsertIntoIndexedArrays {
        authority: context.payer.pubkey(),
        registered_program_pda: None,
    };
    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: vec![
            accounts.to_account_metas(Some(true)),
            vec![
                AccountMeta::new(indexed_array_pubkey, false),
                AccountMeta::new(indexed_array_pubkey, false),
            ],
        ]
        .concat(),
        data: instruction_data.data(),
    };
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &vec![&context.payer],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction.clone())
        .await
        .unwrap();
    let array = AccountZeroCopy::<account_compression::IndexedArrayAccount>::new(
        &mut context,
        indexed_array_pubkey,
    )
    .await;

    let indexed_array = array.deserialized().indexed_array;
    assert_eq!(indexed_array[0].element, [1u8; 32]);
    assert_eq!(indexed_array[1].element, [2u8; 32]);
    assert_eq!(indexed_array[0].merkle_tree_overwrite_sequence_number, 0);
    assert_eq!(indexed_array[1].merkle_tree_overwrite_sequence_number, 0);

    let mut concurrent_merkle_tree = ConcurrentMerkleTree::<Poseidon, 26>::new(
        account_compression::utils::constants::STATE_MERKLE_TREE_HEIGHT,
        account_compression::utils::constants::STATE_MERKLE_TREE_CHANGELOG,
        account_compression::utils::constants::STATE_MERKLE_TREE_ROOTS,
        account_compression::utils::constants::STATE_MERKLE_TREE_CANOPY_DEPTH,
    );
    concurrent_merkle_tree.init().unwrap();
    concurrent_merkle_tree
        .append_batch(&[&[1u8; 32], &[2u8; 32]])
        .unwrap();
    concurrent_merkle_tree.root().unwrap();

    let proof: Vec<[u8; 32]> = reference_merkle_tree
        .get_proof_of_leaf(0, false)
        .unwrap()
        .to_array::<16>()
        .unwrap()
        .to_vec();

    let mut bounded_vec = from_vec(&proof).unwrap();
    concurrent_merkle_tree
        .update_proof_from_canopy(0, &mut bounded_vec)
        .unwrap();
    concurrent_merkle_tree
        .validate_proof(&[1u8; 32], 0, &bounded_vec)
        .unwrap();

    let instructions = [
        account_compression::nullify_leaves::sdk_nullify::create_nullify_instruction(
            vec![2u64].as_slice(),
            vec![0u16].as_slice(),
            vec![0u64].as_slice(),
            vec![proof].as_slice(),
            &context.payer.pubkey(),
            &merkle_tree_pubkey,
            &indexed_array_pubkey,
        ),
    ];

    create_and_send_transaction(&mut context, &instructions, &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    let merkle_tree = AccountZeroCopy::<account_compression::StateMerkleTreeAccount>::new(
        &mut context,
        merkle_tree_pubkey,
    )
    .await;
    reference_merkle_tree.update(&ZERO_BYTES[0], 0).unwrap();
    assert_eq!(
        merkle_tree
            .deserialized()
            .copy_merkle_tree()
            .unwrap()
            .root()
            .unwrap(),
        reference_merkle_tree.root().unwrap()
    );

    let array = AccountZeroCopy::<account_compression::IndexedArrayAccount>::new(
        &mut context,
        indexed_array_pubkey,
    )
    .await;
    let indexed_array = array.deserialized().indexed_array;
    assert_eq!(indexed_array[0].element, [1u8; 32]);
    assert_eq!(
        indexed_array[0].merkle_tree_overwrite_sequence_number,
        merkle_tree
            .deserialized()
            .load_merkle_tree()
            .unwrap()
            .sequence_number as u64
            + account_compression::utils::constants::STATE_MERKLE_TREE_ROOTS as u64
    );
}
