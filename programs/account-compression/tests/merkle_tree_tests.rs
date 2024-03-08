#![cfg(feature = "test-sbf")]

use std::str::FromStr;

use account_compression::{
    self, indexed_array_from_bytes,
    indexed_array_sdk::create_initialize_indexed_array_instruction,
    instructions::insert_two_leaves_transaction::sdk::{
        create_initialize_merkle_tree_instruction, create_insert_leaves_instruction,
    },
    utils::constants::{GROUP_AUTHORITY_SEED, STATE_INDEXED_ARRAY_SIZE},
    GroupAuthority, StateMerkleTreeAccount, ID,
};
use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use ark_ff::BigInteger256;
use ark_serialize::CanonicalDeserialize;
use light_concurrent_merkle_tree::ConcurrentMerkleTree26;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::array::IndexingArray;
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

    let context_keypair = context.payer.insecure_clone();
    let context_pubkey = context.payer.pubkey();

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
    assert_eq!(merkle_tree.deserialized().owner, context_pubkey);
    assert_eq!(merkle_tree.deserialized().delegate, context_pubkey);
    assert_eq!(merkle_tree.deserialized().index, 1);

    // insertions with merkle tree leaves missmatch should fail
    let instruction_data = account_compression::instruction::InsertLeavesIntoMerkleTrees {
        leaves: vec![[1u8; 32], [2u8; 32]],
    };

    let accounts = account_compression::accounts::InsertTwoLeavesParallel {
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

    create_and_send_transaction(
        &mut context,
        &instruction,
        &context_keypair.pubkey(),
        &[&context_keypair],
    )
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
    let context_keypair = context.payer.insecure_clone();
    let context_pubkey = context_keypair.pubkey();

    let indexed_array_keypair = Keypair::new();

    let account_create_ix = create_account_instruction(
        &context_pubkey,
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
        create_initialize_indexed_array_instruction(context_pubkey, indexed_array_pubkey, 1u64);

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
    assert_eq!(array.deserialized().owner, context_pubkey);
    assert_eq!(array.deserialized().delegate, context_pubkey);
    assert_eq!(array.deserialized().index, 1);
    let indexed_array = indexed_array_from_bytes(&array.deserialized().indexed_array);
    let mut default_array =
        IndexingArray::<Poseidon, u16, BigInteger256, STATE_INDEXED_ARRAY_SIZE>::default();
    assert_eq!(indexed_array.elements, default_array.elements);
    assert_eq!(
        indexed_array.current_node_index,
        default_array.current_node_index
    );
    assert_eq!(
        indexed_array.highest_element_index,
        default_array.highest_element_index
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

    let indexed_array = indexed_array_from_bytes(&array.deserialized().indexed_array);
    default_array
        .append(BigInteger256::deserialize_uncompressed_unchecked(&[1u8; 32][..]).unwrap())
        .unwrap();
    default_array
        .append(BigInteger256::deserialize_uncompressed_unchecked(&[2u8; 32][..]).unwrap())
        .unwrap();
    assert_eq!(indexed_array.elements[0], default_array.elements[0]);
    assert_eq!(indexed_array.elements[1], default_array.elements[1]);
}
