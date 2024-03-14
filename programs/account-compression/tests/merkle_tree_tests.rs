#![cfg(feature = "test-sbf")]

use account_compression::{
    self,
    indexed_array_sdk::create_initialize_indexed_array_instruction,
    instructions::append_leaves::sdk::{
        create_initialize_merkle_tree_instruction, create_insert_leaves_instruction,
    },
    utils::constants::{
        STATE_INDEXED_ARRAY_INDICES, STATE_INDEXED_ARRAY_SEQUENCE_THRESHOLD,
        STATE_INDEXED_ARRAY_VALUES,
    },
    Pubkey, StateMerkleTreeAccount, ID,
};
use anchor_lang::{InstructionData, ToAccountMetas};
use light_concurrent_merkle_tree::ConcurrentMerkleTree26;
use light_hasher::{zero_bytes::poseidon::ZERO_BYTES, Poseidon};
use light_test_utils::{
    airdrop_lamports, create_account_instruction, create_and_send_transaction, get_hash_set,
    AccountZeroCopy,
};
use light_utils::bigint::bigint_to_be_bytes_array;
use num_bigint::ToBigUint;
use solana_program_test::{BanksClientError, ProgramTest, ProgramTestContext};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::{Keypair, Signer},
    transaction::Transaction,
};

/// Tests:
/// 1. Functional: Initialize merkle tree
/// 2. Failing: Append with invalid inputs
/// 3. Functional: Append leaves to merkle tree
/// 4. Failing: Append leaves with invalid authority
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

    let payer_pubkey = context.payer.pubkey();

    let merkle_tree_pubkey =
        functional_1_initialize_state_merkle_tree(&mut context, &payer_pubkey, None).await;

    fail_2_append_leaves_with_invalid_inputs(&mut context, &merkle_tree_pubkey).await;

    functional_3_append_leaves_to_merkle_tree(&mut context, &merkle_tree_pubkey).await;

    fail_4_append_leaves_with_invalid_authority(&mut context, &merkle_tree_pubkey).await;
}

async fn functional_1_initialize_state_merkle_tree(
    context: &mut ProgramTestContext,
    payer_pubkey: &Pubkey,
    associated_queue: Option<Pubkey>,
) -> Pubkey {
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

    let instruction = create_initialize_merkle_tree_instruction(
        context.payer.pubkey(),
        merkle_tree_pubkey,
        associated_queue,
    );

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
        context,
        merkle_tree_pubkey,
    )
    .await;
    assert_eq!(merkle_tree.deserialized().owner, *payer_pubkey);
    assert_eq!(merkle_tree.deserialized().delegate, *payer_pubkey);
    assert_eq!(merkle_tree.deserialized().index, 1);
    merkle_tree_keypair.pubkey()
}

pub async fn fail_2_append_leaves_with_invalid_inputs(
    context: &mut ProgramTestContext,
    merkle_tree_pubkey: &Pubkey,
) {
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
            vec![AccountMeta::new(*merkle_tree_pubkey, false)],
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
}

pub async fn functional_3_append_leaves_to_merkle_tree(
    context: &mut ProgramTestContext,
    merkle_tree_pubkey: &Pubkey,
) {
    let payer = context.payer.insecure_clone();
    let old_merkle_tree = AccountZeroCopy::<account_compression::StateMerkleTreeAccount>::new(
        context,
        *merkle_tree_pubkey,
    )
    .await;
    let old_merkle_tree = old_merkle_tree.deserialized().copy_merkle_tree().unwrap();

    let instruction = [create_insert_leaves_instruction(
        vec![[1u8; 32], [2u8; 32]],
        context.payer.pubkey(),
        vec![*merkle_tree_pubkey, *merkle_tree_pubkey],
    )];

    create_and_send_transaction(context, &instruction, &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    let merkle_tree = AccountZeroCopy::<account_compression::StateMerkleTreeAccount>::new(
        context,
        *merkle_tree_pubkey,
    )
    .await;
    let merkle_tree = merkle_tree.deserialized().copy_merkle_tree().unwrap();
    assert_eq!(merkle_tree.next_index, old_merkle_tree.next_index + 2);

    let mut reference_merkle_tree = ConcurrentMerkleTree26::<Poseidon>::new(
        account_compression::utils::constants::STATE_MERKLE_TREE_HEIGHT as usize,
        account_compression::utils::constants::STATE_MERKLE_TREE_CHANGELOG as usize,
        account_compression::utils::constants::STATE_MERKLE_TREE_ROOTS as usize,
        account_compression::utils::constants::STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
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

pub async fn fail_4_append_leaves_with_invalid_authority(
    context: &mut ProgramTestContext,
    merkle_tree_pubkey: &Pubkey,
) {
    let invalid_autority = Keypair::new();
    airdrop_lamports(context, &invalid_autority.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let instruction_data = account_compression::instruction::AppendLeavesToMerkleTrees {
        leaves: vec![[1u8; 32]],
    };

    let accounts = account_compression::accounts::AppendLeaves {
        authority: invalid_autority.pubkey(),
        registered_program_pda: None,
        log_wrapper: account_compression::state::change_log_event::NOOP_PROGRAM_ID,
    };

    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: vec![
            accounts.to_account_metas(Some(true)),
            vec![AccountMeta::new(*merkle_tree_pubkey, false)],
        ]
        .concat(),
        data: instruction_data.data(),
    };
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&invalid_autority.pubkey()),
        &vec![&invalid_autority],
        context.last_blockhash,
    );
    let remaining_accounts_missmatch_error =
        context.banks_client.process_transaction(transaction).await;
    assert!(remaining_accounts_missmatch_error.is_err());
}

/// Tests:
/// 1. Functional: nullify leaf
/// 2. Failing: nullify leaf with invalid leaf index
/// 3. Failing: nullify leaf with invalid leaf queue index
/// 4. Failing: nullify leaf with invalid change log index
/// 5. Functional: nullify other leaf
/// 6. Failing: nullify leaf with indexed array that is not associated with the merkle tree
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
    let indexed_array_keypair = Keypair::new();

    let merkle_tree_pubkey = functional_1_initialize_state_merkle_tree(
        &mut context,
        &payer_pubkey,
        Some(indexed_array_keypair.pubkey()),
    )
    .await;

    let indexed_array_pubkey = functional_1_initialize_indexed_array(
        &mut context,
        &payer_pubkey,
        &indexed_array_keypair,
        Some(merkle_tree_pubkey),
    )
    .await;
    let other_merkle_tree_pubkey = functional_1_initialize_state_merkle_tree(
        &mut context,
        &payer_pubkey,
        Some(indexed_array_keypair.pubkey()),
    )
    .await;
    let invalid_indexed_array_keypair = Keypair::new();
    let invalid_indexed_array_pubkey = functional_1_initialize_indexed_array(
        &mut context,
        &payer_pubkey,
        &invalid_indexed_array_keypair,
        Some(other_merkle_tree_pubkey),
    )
    .await;

    functional_3_append_leaves_to_merkle_tree(&mut context, &merkle_tree_pubkey).await;

    let elements = vec![[1u8; 32], [2u8; 32]];

    insert_into_indexed_arrays(&elements, &payer, &indexed_array_pubkey, &mut context)
        .await
        .unwrap();

    let mut reference_merkle_tree = light_merkle_tree_reference::MerkleTree::<Poseidon>::new(
        account_compression::utils::constants::STATE_MERKLE_TREE_HEIGHT as usize,
        account_compression::utils::constants::STATE_MERKLE_TREE_ROOTS as usize,
        account_compression::utils::constants::STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
    )
    .unwrap();
    reference_merkle_tree.append(&elements[0]).unwrap();
    reference_merkle_tree.append(&elements[1]).unwrap();

    let element_index = reference_merkle_tree.get_leaf_index(&elements[0]).unwrap() as u64;
    nullify(
        &mut context,
        &merkle_tree_pubkey,
        &indexed_array_pubkey,
        &mut reference_merkle_tree,
        &elements[0],
        2,
        0,
        element_index,
    )
    .await
    .unwrap();

    // nullify with invalid leaf index
    let invalid_element_index = 0;
    let valid_changelog_index = 3;
    let valid_leaf_queue_index = 1;
    nullify(
        &mut context,
        &merkle_tree_pubkey,
        &indexed_array_pubkey,
        &mut reference_merkle_tree,
        &elements[1],
        valid_changelog_index,
        valid_leaf_queue_index,
        invalid_element_index,
    )
    .await
    .unwrap_err();
    let valid_element_index = 1;
    let invalid_leaf_queue_index = 0;
    nullify(
        &mut context,
        &merkle_tree_pubkey,
        &indexed_array_pubkey,
        &mut reference_merkle_tree,
        &elements[1],
        valid_changelog_index,
        invalid_leaf_queue_index,
        valid_element_index,
    )
    .await
    .unwrap_err();
    let invalid_change_log_index = 0;
    nullify(
        &mut context,
        &merkle_tree_pubkey,
        &indexed_array_pubkey,
        &mut reference_merkle_tree,
        &elements[1],
        invalid_change_log_index,
        valid_leaf_queue_index,
        valid_element_index,
    )
    .await
    .unwrap_err();
    nullify(
        &mut context,
        &merkle_tree_pubkey,
        &indexed_array_pubkey,
        &mut reference_merkle_tree,
        &elements[1],
        valid_changelog_index,
        valid_leaf_queue_index,
        valid_element_index,
    )
    .await
    .unwrap();

    nullify(
        &mut context,
        &merkle_tree_pubkey,
        &invalid_indexed_array_pubkey,
        &mut reference_merkle_tree,
        &elements[0],
        2,
        0,
        element_index,
    )
    .await
    .unwrap_err();
}

pub async fn nullify(
    context: &mut ProgramTestContext,
    merkle_tree_pubkey: &Pubkey,
    indexed_array_pubkey: &Pubkey,
    reference_merkle_tree: &mut light_merkle_tree_reference::MerkleTree<Poseidon>,
    element: &[u8; 32],
    change_log_index: u64,
    leaf_queue_index: u16,
    element_index: u64,
) -> std::result::Result<(), BanksClientError> {
    let proof: Vec<[u8; 32]> = reference_merkle_tree
        .get_proof_of_leaf(element_index as usize, false)
        .unwrap()
        .to_array::<16>()
        .unwrap()
        .to_vec();

    let instructions = [
        account_compression::nullify_leaves::sdk_nullify::create_nullify_instruction(
            vec![change_log_index].as_slice(),
            vec![leaf_queue_index.clone()].as_slice(),
            vec![element_index as u64].as_slice(),
            vec![proof].as_slice(),
            &context.payer.pubkey(),
            merkle_tree_pubkey,
            indexed_array_pubkey,
        ),
    ];
    let payer = context.payer.insecure_clone();

    create_and_send_transaction(context, &instructions, &payer.pubkey(), &[&payer]).await?;

    let merkle_tree = AccountZeroCopy::<account_compression::StateMerkleTreeAccount>::new(
        context,
        *merkle_tree_pubkey,
    )
    .await;
    reference_merkle_tree
        .update(&ZERO_BYTES[0], element_index as usize)
        .unwrap();
    assert_eq!(
        merkle_tree
            .deserialized()
            .copy_merkle_tree()
            .unwrap()
            .root()
            .unwrap(),
        reference_merkle_tree.root().unwrap()
    );

    let indexed_array = unsafe {
        get_hash_set::<u16, account_compression::IndexedArrayAccount>(
            context,
            *indexed_array_pubkey,
        )
        .await
    };
    let indexed_array = indexed_array.lock().await;
    let array_element = indexed_array
        .by_value_index(leaf_queue_index.into())
        .unwrap();
    assert_eq!(&array_element.value_bytes(), element);
    assert_eq!(
        array_element.sequence_number(),
        Some(
            merkle_tree
                .deserialized()
                .load_merkle_tree()
                .unwrap()
                .sequence_number
                + account_compression::utils::constants::STATE_MERKLE_TREE_ROOTS as usize
        )
    );
    Ok(())
}

/// Tests:
/// 1. Functional: Initialize indexed array
/// 2. Functional: Insert into indexed array
/// 3. Failing: Insert the same elements into indexed array again (3 and 1 element(s))
/// 4. Failing: Insert into indexed array with invalid authority
/// 5. Functional: Insert one element into into indexed array
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
    let indexed_array_pubkey = functional_1_initialize_indexed_array(
        &mut context,
        &payer_pubkey,
        &indexed_array_keypair,
        None,
    )
    .await;

    functional_2_test_insert_into_indexed_arrays(&mut context, &indexed_array_pubkey).await;

    fail_3_insert_same_elements_into_indexed_array(
        &mut context,
        &indexed_array_pubkey,
        vec![[3u8; 32], [1u8; 32], [1u8; 32]],
    )
    .await;
    fail_3_insert_same_elements_into_indexed_array(
        &mut context,
        &indexed_array_pubkey,
        vec![[1u8; 32]],
    )
    .await;
    fail_4_insert_with_invalid_signer(&mut context, &indexed_array_pubkey, vec![[3u8; 32]]).await;

    functional_5_test_insert_into_indexed_arrays(&mut context, &indexed_array_pubkey).await;
}

async fn functional_1_initialize_indexed_array(
    context: &mut ProgramTestContext,
    payer_pubkey: &Pubkey,
    indexed_array_keypair: &Keypair,
    associated_merkle_tree: Option<Pubkey>,
) -> Pubkey {
    let size = account_compression::IndexedArrayAccount::size(
        account_compression::utils::constants::STATE_INDEXED_ARRAY_INDICES as usize,
        account_compression::utils::constants::STATE_INDEXED_ARRAY_VALUES as usize,
    )
    .unwrap();
    let account_create_ix = create_account_instruction(
        payer_pubkey,
        size,
        context
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(size),
        &ID,
        Some(&indexed_array_keypair),
    );
    let indexed_array_pubkey = indexed_array_keypair.pubkey();

    let instruction = create_initialize_indexed_array_instruction(
        *payer_pubkey,
        indexed_array_pubkey,
        1u64,
        associated_merkle_tree,
        STATE_INDEXED_ARRAY_INDICES,
        STATE_INDEXED_ARRAY_VALUES,
        STATE_INDEXED_ARRAY_SEQUENCE_THRESHOLD,
    );

    let transaction = Transaction::new_signed_with_payer(
        &[account_create_ix, instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer, &indexed_array_keypair],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction.clone())
        .await
        .unwrap();
    let indexed_array = AccountZeroCopy::<account_compression::IndexedArrayAccount>::new(
        context,
        indexed_array_pubkey,
    )
    .await
    .deserialized();
    assert_eq!(
        indexed_array.associated_merkle_tree,
        associated_merkle_tree.unwrap_or(Pubkey::default())
    );
    assert_eq!(indexed_array.index, 1);
    assert_eq!(indexed_array.owner, *payer_pubkey);
    assert_eq!(indexed_array.delegate, *payer_pubkey);

    indexed_array_pubkey
}

async fn functional_2_test_insert_into_indexed_arrays(
    context: &mut ProgramTestContext,
    indexed_array_pubkey: &Pubkey,
) {
    let payer = context.payer.insecure_clone();

    let elements = vec![[1_u8; 32], [2_u8; 32]];
    insert_into_indexed_arrays(&elements, &payer, indexed_array_pubkey, context)
        .await
        .unwrap();
    let array = unsafe {
        get_hash_set::<u16, account_compression::IndexedArrayAccount>(
            context,
            *indexed_array_pubkey,
        )
        .await
    };
    let array = array.lock().await;
    let array_element_0 = array.by_value_index(0).unwrap();
    assert_eq!(array_element_0.value_bytes(), [1u8; 32]);
    assert_eq!(array_element_0.sequence_number(), None);
    let array_element_1 = array.by_value_index(1).unwrap();
    assert_eq!(array_element_1.value_bytes(), [2u8; 32]);
    assert_eq!(array_element_1.sequence_number(), None);
}

async fn fail_3_insert_same_elements_into_indexed_array(
    context: &mut ProgramTestContext,
    indexed_array_pubkey: &Pubkey,
    elements: Vec<[u8; 32]>,
) {
    let payer = context.payer.insecure_clone();

    insert_into_indexed_arrays(&elements, &payer, indexed_array_pubkey, context)
        .await
        .unwrap_err();
}

async fn fail_4_insert_with_invalid_signer(
    context: &mut ProgramTestContext,
    indexed_array_pubkey: &Pubkey,
    elements: Vec<[u8; 32]>,
) {
    let invalid_signer = Keypair::new();
    airdrop_lamports(context, &invalid_signer.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    insert_into_indexed_arrays(&elements, &invalid_signer, indexed_array_pubkey, context)
        .await
        .unwrap_err();
}

async fn functional_5_test_insert_into_indexed_arrays(
    context: &mut ProgramTestContext,
    indexed_array_pubkey: &Pubkey,
) {
    let payer = context.payer.insecure_clone();

    let element = 3_u32.to_biguint().unwrap();
    let elements = vec![bigint_to_be_bytes_array(&element).unwrap()];
    insert_into_indexed_arrays(&elements, &payer, indexed_array_pubkey, context)
        .await
        .unwrap();
    let array = unsafe {
        get_hash_set::<u16, account_compression::IndexedArrayAccount>(
            context,
            *indexed_array_pubkey,
        )
        .await
    };
    let array = array.lock().await;
    let array_element = array.by_value_index(2).unwrap();
    assert_eq!(array_element.value_biguint(), element);
    assert_eq!(array_element.sequence_number(), None);
}

async fn insert_into_indexed_arrays(
    elements: &Vec<[u8; 32]>,
    payer: &Keypair,
    indexed_array_pubkey: &Pubkey,
    context: &mut ProgramTestContext,
) -> std::result::Result<(), BanksClientError> {
    let instruction_data = account_compression::instruction::InsertIntoIndexedArrays {
        elements: elements.to_vec(),
    };
    let accounts = account_compression::accounts::InsertIntoIndexedArrays {
        authority: payer.pubkey(),
        registered_program_pda: None,
    };
    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: vec![
            accounts.to_account_metas(Some(true)),
            vec![AccountMeta::new(*indexed_array_pubkey, false); elements.len()],
        ]
        .concat(),
        data: instruction_data.data(),
    };
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &vec![payer],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction.clone())
        .await
}
