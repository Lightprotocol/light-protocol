#![cfg(feature = "test-sbf")]

use account_compression::{
    accounts,
    errors::AccountCompressionErrorCode,
    initialize_address_merkle_tree::AccountLoader,
    instruction::{self, InsertAddresses, UpdateAddressMerkleTree},
    sdk::create_initialize_address_merkle_tree_and_queue_instruction,
    state::AddressMerkleTreeAccount,
    utils::constants::{ADDRESS_MERKLE_TREE_CANOPY_DEPTH, ADDRESS_MERKLE_TREE_HEIGHT},
    AddressMerkleTreeConfig, AddressQueueAccount, AddressQueueConfig, ID,
};
use anchor_lang::{InstructionData, Lamports};
use anchor_lang::{Key, ToAccountMetas};
use light_bounded_vec::BoundedVec;
use light_concurrent_merkle_tree::ConcurrentMerkleTree;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::{array::IndexedArray, errors::IndexedMerkleTreeError, reference};
use light_test_utils::{create_account_instruction, get_hash_set, AccountZeroCopy};
use light_utils::bigint::bigint_to_be_bytes_array;
use num_bigint::ToBigUint;
use solana_program_test::{
    BanksClientError, BanksTransactionResultWithMetadata, ProgramTest, ProgramTestContext,
};
use solana_sdk::{
    account::AccountSharedData,
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction, InstructionError},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::{Transaction, TransactionError},
};
use thiserror::Error;

#[derive(Error, Debug)]
enum RelayerUpdateError {}

pub async fn create_address_merkle_tree_and_queue_account(
    payer: &Keypair,
    context: &mut ProgramTestContext,
    address_merkle_tree_keypair: &Keypair,
    address_queue_keypair: &Keypair,
) {
    let size = account_compression::AddressQueueAccount::size(
        account_compression::utils::constants::ADDRESS_QUEUE_INDICES as usize,
        account_compression::utils::constants::ADDRESS_QUEUE_VALUES as usize,
    )
    .unwrap();
    let account_create_ix = crate::create_account_instruction(
        &payer.pubkey(),
        size,
        context
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(size),
        &account_compression::ID,
        Some(address_queue_keypair),
    );

    let mt_account_create_ix = crate::create_account_instruction(
        &payer.pubkey(),
        account_compression::AddressMerkleTreeAccount::LEN,
        context
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(account_compression::AddressMerkleTreeAccount::LEN),
        &account_compression::ID,
        Some(address_merkle_tree_keypair),
    );

    let instruction = create_initialize_address_merkle_tree_and_queue_instruction(
        1u64,
        payer.pubkey(),
        None,
        address_merkle_tree_keypair.pubkey(),
        address_queue_keypair.pubkey(),
        AddressMerkleTreeConfig::default(),
        AddressQueueConfig::default(),
    );
    let transaction = Transaction::new_signed_with_payer(
        &[account_create_ix, mt_account_create_ix, instruction],
        Some(&payer.pubkey()),
        &vec![&payer, &address_queue_keypair, &address_merkle_tree_keypair],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction.clone())
        .await
        .unwrap();
}

async fn insert_addresses(
    context: &mut ProgramTestContext,
    address_queue_pubkey: Pubkey,
    address_merkle_tree_pubkey: Pubkey,
    addresses: Vec<[u8; 32]>,
) -> Result<(), BanksClientError> {
    let num_addresses = addresses.len();
    let instruction_data = InsertAddresses { addresses };
    let accounts = account_compression::accounts::InsertAddresses {
        fee_payer: context.payer.pubkey(),
        authority: context.payer.pubkey(),
        registered_program_pda: None,
        system_program: system_program::ID,
    };
    let insert_ix = Instruction {
        program_id: ID,
        accounts: [
            accounts.to_account_metas(Some(true)),
            vec![
                vec![
                    AccountMeta::new(address_queue_pubkey, false),
                    AccountMeta::new(address_merkle_tree_pubkey, false),
                ];
                num_addresses
            ]
            .iter()
            .flat_map(|x| x.to_vec())
            .collect::<Vec<AccountMeta>>(),
        ]
        .concat(),
        data: instruction_data.data(),
    };
    let transaction = Transaction::new_signed_with_payer(
        &[insert_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(transaction).await
}

async fn update_merkle_tree(
    context: &mut ProgramTestContext,
    address_queue_pubkey: Pubkey,
    address_merkle_tree_pubkey: Pubkey,
    value: u16,
    next_index: u64,
    low_address_index: u64,
    low_address_value: [u8; 32],
    low_address_next_index: u64,
    low_address_next_value: [u8; 32],
    low_address_proof: [[u8; 32]; 16],
) -> Result<BanksTransactionResultWithMetadata, BanksClientError> {
    let changelog_index = {
        // TODO: figure out why I get an invalid memory reference error here when I try to replace 183-190 with this
        let address_merkle_tree =
            AccountZeroCopy::<AddressMerkleTreeAccount>::new(context, address_merkle_tree_pubkey)
                .await;

        let address_merkle_tree = &address_merkle_tree
            .deserialized()
            .load_merkle_tree()
            .unwrap();
        let changelog_index = address_merkle_tree.merkle_tree.changelog_index();
        changelog_index
    };

    let instruction_data = UpdateAddressMerkleTree {
        changelog_index: changelog_index as u16,
        value,
        next_index,
        low_address_index,
        low_address_value,
        low_address_next_index,
        low_address_next_value,
        low_address_proof,
    };
    let update_ix = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(context.payer.pubkey(), true),
            AccountMeta::new(address_queue_pubkey, false),
            AccountMeta::new(address_merkle_tree_pubkey, false),
        ],
        data: instruction_data.data(),
    };
    let transaction = Transaction::new_signed_with_payer(
        &[update_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction_with_metadata(transaction)
        .await
}

async fn relayer_update(
    context: &mut ProgramTestContext,
    address_queue_pubkey: Pubkey,
    address_merkle_tree_pubkey: Pubkey,
) -> Result<(), RelayerUpdateError> {
    let mut relayer_indexing_array = Box::new(IndexedArray::<
        Poseidon,
        usize,
        // This is not a correct value you would normally use in relayer, A
        // correct size would be number of leaves which the merkle tree can fit
        // (`MERKLE_TREE_LEAVES`). Allocating an indexing array for over 4 mln
        // elements ain't easy and is not worth doing here.
        200,
    >::default());
    relayer_indexing_array.init().unwrap();
    let mut relayer_merkle_tree = Box::new(
        reference::IndexedMerkleTree::<Poseidon, usize>::new(
            ADDRESS_MERKLE_TREE_HEIGHT as usize,
            ADDRESS_MERKLE_TREE_CANOPY_DEPTH as usize,
        )
        .unwrap(),
    );
    relayer_merkle_tree.init().unwrap();

    let mut update_errors: Vec<TransactionError> = Vec::new();

    loop {
        let address_merkle_tree =
            AccountZeroCopy::<AddressMerkleTreeAccount>::new(context, address_merkle_tree_pubkey)
                .await;
        let mut address_merkle_tree_deserialized = address_merkle_tree.deserialized().clone();
        let address_merkle_tree = address_merkle_tree_deserialized
            .load_merkle_tree_mut()
            .unwrap();
        assert_eq!(
            relayer_merkle_tree.root(),
            address_merkle_tree.merkle_tree.root(),
        );
        let address_queue = unsafe {
            get_hash_set::<u16, AddressQueueAccount>(context, address_queue_pubkey).await
        };

        let address = address_queue.first_no_seq().unwrap();
        if address.is_none() {
            break;
        }
        let (address, address_hashset_index) = address.unwrap();
        // Create new element from the dequeued value.
        let (old_low_address, old_low_address_next_value) = relayer_indexing_array
            .find_low_element(&address.value_biguint())
            .unwrap();
        let address_bundle = relayer_indexing_array
            .new_element_with_low_element_index(old_low_address.index, &address.value_biguint())
            .unwrap();

        // Get the Merkle proof for updating low element.
        let low_address_proof = relayer_merkle_tree
            .get_proof_of_leaf(usize::from(old_low_address.index), false)
            .unwrap();

        let reference_proof = relayer_merkle_tree
            .get_proof_of_leaf(old_low_address.index, false)
            .unwrap();
        let array: [[u8; 32]; 16] = reference_proof.to_array::<16>().unwrap();
        let mut bounded_vec = BoundedVec::with_capacity(26);
        for i in 0..16 {
            bounded_vec.push(array[i]).unwrap();
        }
        address_merkle_tree
            .merkle_tree
            .update(
                address_merkle_tree.merkle_tree.changelog_index(),
                address_bundle.new_element.clone(),
                old_low_address.clone(),
                old_low_address_next_value.clone(),
                &mut bounded_vec,
            )
            .unwrap();

        // Update on-chain tree.
        let update_successful = match update_merkle_tree(
            context,
            address_queue_pubkey,
            address_merkle_tree_pubkey,
            address_hashset_index,
            address_bundle.new_element.next_index as u64,
            old_low_address.index as u64,
            bigint_to_be_bytes_array(&old_low_address.value).unwrap(),
            old_low_address.next_index as u64,
            bigint_to_be_bytes_array(&old_low_address_next_value).unwrap(),
            low_address_proof.to_array().unwrap(),
        )
        .await
        .unwrap()
        .result
        {
            Ok(_) => true,
            Err(e) => {
                update_errors.push(e);
                break;
            }
        };

        if update_successful {
            relayer_merkle_tree
                .update(
                    &address_bundle.new_low_element,
                    &address_bundle.new_element,
                    &address_bundle.new_element_next_value,
                )
                .unwrap();
            relayer_indexing_array
                .append_with_low_element_index(
                    address_bundle.new_low_element.index,
                    &address_bundle.new_element.value,
                )
                .unwrap();
        }
    }

    if update_errors.is_empty() {
        Ok(())
    } else {
        panic!("Errors: {:?}", update_errors);
    }
}

// TODO: enable address Merkle tree tests
/// Tests insertion of addresses to the queue, dequeuing and Merkle tree update.
#[tokio::test]
async fn test_address_queue() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    program_test.set_compute_max_units(1_400_000u64);

    let mut context = program_test.start_with_context().await;

    let payer = context.payer.insecure_clone();

    let address_merkle_tree_keypair = Keypair::new();
    let address_queue_keypair = Keypair::new();
    create_address_merkle_tree_and_queue_account(
        &payer,
        &mut context,
        &address_merkle_tree_keypair,
        &address_queue_keypair,
    )
    .await;

    // Insert a pair of addresses.
    let address1 = 30_u32.to_biguint().unwrap();
    let address2 = 10_u32.to_biguint().unwrap();
    let addresses: Vec<[u8; 32]> = vec![
        bigint_to_be_bytes_array(&address1).unwrap(),
        bigint_to_be_bytes_array(&address2).unwrap(),
    ];
    insert_addresses(
        &mut context,
        address_queue_keypair.pubkey(),
        address_merkle_tree_keypair.pubkey(),
        addresses,
    )
    .await
    .unwrap();

    let address_merkle_tree = AccountZeroCopy::<AddressMerkleTreeAccount>::new(
        &mut context,
        address_merkle_tree_keypair.pubkey(),
    )
    .await;
    let address_merkle_tree = &address_merkle_tree
        .deserialized()
        .copy_merkle_tree()
        .unwrap();

    let address_queue = unsafe {
        get_hash_set::<u16, AddressQueueAccount>(&mut context, address_queue_keypair.pubkey()).await
    };

    assert_eq!(
        address_queue
            .contains(&address1, address_merkle_tree.0.merkle_tree.sequence_number)
            .unwrap(),
        true
    );
    assert_eq!(
        address_queue
            .contains(&address2, address_merkle_tree.0.merkle_tree.sequence_number)
            .unwrap(),
        true
    );
    relayer_update(
        &mut context,
        address_queue_keypair.pubkey(),
        address_merkle_tree_keypair.pubkey(),
    )
    .await
    .unwrap();
}

/// Try to insert an address to the tree while pointing to an invalid low
/// address.
///
/// Such invalid insertion needs to be performed manually, without relayer's
/// help (which would always insert that nullifier correctly).
#[tokio::test]
async fn test_insert_invalid_low_element() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    let mut context = program_test.start_with_context().await;

    let payer = context.payer.insecure_clone();

    let address_merkle_tree_keypair = Keypair::new();
    let address_queue_keypair = Keypair::new();
    create_address_merkle_tree_and_queue_account(
        &payer,
        &mut context,
        &address_merkle_tree_keypair,
        &address_queue_keypair,
    )
    .await;

    // Local indexing array and queue. We will use them to get the correct
    // elements and Merkle proofs, which we will modify later, to pass invalid
    // values. 😈
    let mut local_indexed_array = Box::new(IndexedArray::<
        Poseidon,
        usize,
        // This is not a correct value you would normally use in relayer, A
        // correct size would be number of leaves which the merkle tree can fit
        // (`MERKLE_TREE_LEAVES`). Allocating an indexing array for over 4 mln
        // elements ain't easy and is not worth doing here.
        200,
    >::default());
    let mut local_merkle_tree = Box::new(
        reference::IndexedMerkleTree::<Poseidon, usize>::new(
            ADDRESS_MERKLE_TREE_HEIGHT as usize,
            ADDRESS_MERKLE_TREE_CANOPY_DEPTH as usize,
        )
        .unwrap(),
    );

    // Insert a pair of addresses, correctly. Just do it with relayer.
    let address1 = 30_u32.to_biguint().unwrap();
    let address2 = 10_u32.to_biguint().unwrap();
    let addresses: Vec<[u8; 32]> = vec![
        bigint_to_be_bytes_array(&address1).unwrap(),
        bigint_to_be_bytes_array(&address2).unwrap(),
    ];
    insert_addresses(
        &mut context,
        address_queue_keypair.pubkey(),
        address_merkle_tree_keypair.pubkey(),
        addresses,
    )
    .await
    .unwrap();
    relayer_update(
        &mut context,
        address_queue_keypair.pubkey(),
        address_merkle_tree_keypair.pubkey(),
    )
    .await
    .unwrap();

    // Insert the same pair to the local array and MT.
    let bundle = local_indexed_array.append(&address1).unwrap();
    local_merkle_tree
        .update(
            &bundle.new_low_element,
            &bundle.new_element,
            &bundle.new_element_next_value,
        )
        .unwrap();
    let bundle = local_indexed_array.append(&address2).unwrap();
    local_merkle_tree
        .update(
            &bundle.new_low_element,
            &bundle.new_element,
            &bundle.new_element_next_value,
        )
        .unwrap();

    // Try inserting address 20, while pointing to index 1 (value 30) as low
    // element. Point to index 2 (value 10) as next value.
    // Therefore, the new element is lower than the supposed low element.
    let address3 = 20_u32.to_biguint().unwrap();
    let addresses: Vec<[u8; 32]> = vec![bigint_to_be_bytes_array(&address3).unwrap()];
    insert_addresses(
        &mut context,
        address_queue_keypair.pubkey(),
        address_merkle_tree_keypair.pubkey(),
        addresses,
    )
    .await
    .unwrap();
    let address_queue = unsafe {
        get_hash_set::<u16, AddressQueueAccount>(&mut context, address_queue_keypair.pubkey()).await
    };
    let (_, index) = address_queue
        .find_element(&address3, None)
        .unwrap()
        .unwrap();

    // (Invalid) index of the next address.
    let next_index = 2_usize;
    // (Invalid) low nullifier.
    let low_element = local_indexed_array.get(1).cloned().unwrap();
    let low_element_next_value = local_indexed_array
        .get(usize::from(low_element.next_index))
        .cloned()
        .unwrap()
        .value;
    let low_element_proof = local_merkle_tree.get_proof_of_leaf(1, false).unwrap();
    assert_eq!(
        update_merkle_tree(
            &mut context,
            address_queue_keypair.pubkey(),
            address_merkle_tree_keypair.pubkey(),
            index as u16,
            next_index as u64,
            low_element.index as u64,
            bigint_to_be_bytes_array(&low_element.value).unwrap(),
            low_element.next_index as u64,
            bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
            low_element_proof.to_array().unwrap(),
        )
        .await
        .unwrap()
        .result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(
                IndexedMerkleTreeError::LowElementGreaterOrEqualToNewElement.into()
            )
        ))
    );

    // Try inserting address 50, while pointing to index 0 as low element.
    // Therefore, the new element is greater than next element.
    let address4 = 50_u32.to_biguint().unwrap();
    let addresses: Vec<[u8; 32]> = vec![bigint_to_be_bytes_array(&address4).unwrap()];
    insert_addresses(
        &mut context,
        address_queue_keypair.pubkey(),
        address_merkle_tree_keypair.pubkey(),
        addresses,
    )
    .await
    .unwrap();
    let address_queue = unsafe {
        get_hash_set::<u16, AddressQueueAccount>(&mut context, address_queue_keypair.pubkey()).await
    };
    let (_, index) = address_queue
        .find_element(&address4, None)
        .unwrap()
        .unwrap();
    // Index of our new nullifier in the queue.
    // let queue_index = 1_u16;
    // (Invalid) index of the next address.
    let next_index = 1_usize;
    // (Invalid) low nullifier.
    let low_element = local_indexed_array.get(0).cloned().unwrap();
    let low_element_next_value = local_indexed_array
        .get(usize::from(low_element.next_index))
        .cloned()
        .unwrap()
        .value;
    let low_element_proof = local_merkle_tree.get_proof_of_leaf(0, false).unwrap();
    assert_eq!(
        update_merkle_tree(
            &mut context,
            address_queue_keypair.pubkey(),
            address_merkle_tree_keypair.pubkey(),
            index as u16,
            next_index as u64,
            low_element.index as u64,
            bigint_to_be_bytes_array(&low_element.value).unwrap(),
            low_element.next_index as u64,
            bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
            low_element_proof.to_array().unwrap(),
        )
        .await
        .unwrap()
        .result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(
                IndexedMerkleTreeError::NewElementGreaterOrEqualToNextElement.into()
            )
        ))
    );
}

#[tokio::test]
async fn test_address_merkle_tree_and_queue_rollover() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    let mut context = program_test.start_with_context().await;

    let payer = context.payer.insecure_clone();

    let address_merkle_tree_keypair = Keypair::new();
    let address_queue_keypair = Keypair::new();
    create_address_merkle_tree_and_queue_account(
        &payer,
        &mut context,
        &address_merkle_tree_keypair,
        &address_queue_keypair,
    )
    .await;

    let address_merkle_tree_keypair_2 = Keypair::new();
    let address_queue_keypair_2 = Keypair::new();
    create_address_merkle_tree_and_queue_account(
        &payer,
        &mut context,
        &address_merkle_tree_keypair_2,
        &address_queue_keypair_2,
    )
    .await;
    let merkle_tree_config = AddressMerkleTreeConfig::default();
    let required_next_index = 2u64.pow(26) * merkle_tree_config.rollover_threshold.unwrap() / 100;
    let failing_next_index = required_next_index - 1;

    let new_queue_keypair = Keypair::new();
    let new_address_merkle_tree_keypair = Keypair::new();

    let res = perform_address_merkle_tree_roll_over(
        &mut context,
        &new_queue_keypair,
        &new_address_merkle_tree_keypair,
        &address_merkle_tree_keypair.pubkey(),
        &address_queue_keypair.pubkey(),
    )
    .await
    .unwrap();
    assert_eq!(
        res.result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            2,
            InstructionError::Custom(AccountCompressionErrorCode::NotReadyForRollover.into())
        ))
    );
    let lamports_queue_accounts = context
        .banks_client
        .get_account(address_queue_keypair.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports
        + context
            .banks_client
            .get_account(address_merkle_tree_keypair.pubkey())
            .await
            .unwrap()
            .unwrap()
            .lamports
            * 2;
    set_address_merkle_tree_next_index(
        &mut context,
        &address_merkle_tree_keypair.pubkey(),
        failing_next_index,
        lamports_queue_accounts,
    )
    .await;
    let res = perform_address_merkle_tree_roll_over(
        &mut context,
        &new_queue_keypair,
        &new_address_merkle_tree_keypair,
        &address_merkle_tree_keypair.pubkey(),
        &address_queue_keypair.pubkey(),
    )
    .await
    .unwrap();
    assert_eq!(
        res.result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            2,
            InstructionError::Custom(AccountCompressionErrorCode::NotReadyForRollover.into())
        ))
    );

    set_address_merkle_tree_next_index(
        &mut context,
        &address_merkle_tree_keypair.pubkey(),
        required_next_index,
        lamports_queue_accounts,
    )
    .await;

    let res = perform_address_merkle_tree_roll_over(
        &mut context,
        &new_queue_keypair,
        &new_address_merkle_tree_keypair,
        &address_merkle_tree_keypair.pubkey(),
        &address_queue_keypair_2.pubkey(),
    )
    .await
    .unwrap();
    assert_eq!(
        res.result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            2,
            InstructionError::Custom(
                AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated.into()
            )
        ))
    );
    let res = perform_address_merkle_tree_roll_over(
        &mut context,
        &new_queue_keypair,
        &new_address_merkle_tree_keypair,
        &address_merkle_tree_keypair_2.pubkey(),
        &address_queue_keypair.pubkey(),
    )
    .await
    .unwrap();
    assert_eq!(
        res.result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            2,
            InstructionError::Custom(
                AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated.into()
            )
        ))
    );
    let signer_prior_balance = context
        .banks_client
        .get_account(payer.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;
    perform_address_merkle_tree_roll_over(
        &mut context,
        &new_queue_keypair,
        &new_address_merkle_tree_keypair,
        &address_merkle_tree_keypair.pubkey(),
        &address_queue_keypair.pubkey(),
    )
    .await
    .unwrap()
    .result
    .unwrap();
    assert_rolled_over_address_merkle_tree_and_queue(
        &mut context,
        &signer_prior_balance,
        &address_merkle_tree_keypair.pubkey(),
        &address_queue_keypair.pubkey(),
        &new_address_merkle_tree_keypair.pubkey(),
        &new_queue_keypair.pubkey(),
    )
    .await;

    let failing_new_nullifier_queue_keypair = Keypair::new();
    let failing_new_state_merkle_tree_keypair = Keypair::new();

    let res = perform_address_merkle_tree_roll_over(
        &mut context,
        &failing_new_nullifier_queue_keypair,
        &failing_new_state_merkle_tree_keypair,
        &address_merkle_tree_keypair.pubkey(),
        &address_queue_keypair.pubkey(),
    )
    .await
    .unwrap();
    assert_eq!(
        res.result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            2,
            InstructionError::Custom(
                AccountCompressionErrorCode::MerkleTreeAlreadyRolledOver.into()
            )
        ))
    );
}

use solana_sdk::account::WritableAccount;
pub async fn set_address_merkle_tree_next_index(
    context: &mut ProgramTestContext,
    merkle_tree_pubkey: &Pubkey,
    next_index: u64,
    lamports: u64,
) {
    // is in range 8 -9 in concurrent mt
    // offset for next index
    let offset_start = 6 * 8 + 8 + 4 * 32 + 8 * 8;
    let offset_end = offset_start + 8;
    let mut merkle_tree = context
        .banks_client
        .get_account(*merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap();
    merkle_tree.data[offset_start..offset_end].copy_from_slice(&next_index.to_le_bytes());
    let mut account_share_data = AccountSharedData::from(merkle_tree);
    account_share_data.set_lamports(lamports);
    context.set_account(merkle_tree_pubkey, &account_share_data);
    let merkle_tree = context
        .banks_client
        .get_account(*merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap();
    let data_in_offset = u64::from_le_bytes(
        merkle_tree.data[offset_start..offset_end]
            .try_into()
            .unwrap(),
    );
    assert_eq!(data_in_offset, next_index);
}

pub async fn perform_address_merkle_tree_roll_over(
    context: &mut ProgramTestContext,
    new_queue_keypair: &Keypair,
    new_address_merkle_tree_keypair: &Keypair,
    old_merkle_tree_pubkey: &Pubkey,
    old_queue_pubkey: &Pubkey,
) -> Result<BanksTransactionResultWithMetadata, BanksClientError> {
    let payer = context.payer.insecure_clone();
    let size = account_compression::AddressQueueAccount::size(
        account_compression::utils::constants::ADDRESS_QUEUE_INDICES as usize,
        account_compression::utils::constants::ADDRESS_QUEUE_VALUES as usize,
    )
    .unwrap();
    let account_create_ix = crate::create_account_instruction(
        &payer.pubkey(),
        size,
        context
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(size),
        &account_compression::ID,
        Some(new_queue_keypair),
    );

    let mt_account_create_ix = crate::create_account_instruction(
        &payer.pubkey(),
        account_compression::AddressMerkleTreeAccount::LEN,
        context
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(account_compression::AddressMerkleTreeAccount::LEN),
        &account_compression::ID,
        Some(new_address_merkle_tree_keypair),
    );
    let instruction_data = instruction::RolloverAddressMerkleTreeAndQueue {};
    let accounts = accounts::RolloverAddressMerkleTreeAndQueue {
        fee_payer: context.payer.pubkey(),
        authority: context.payer.pubkey(),
        registered_program_pda: None,
        new_address_merkle_tree: new_address_merkle_tree_keypair.pubkey(),
        new_queue: new_queue_keypair.pubkey(),
        old_address_merkle_tree: *old_merkle_tree_pubkey,
        old_queue: *old_queue_pubkey,
    };
    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: vec![accounts.to_account_metas(Some(true))].concat(),
        data: instruction_data.data(),
    };
    let blockhash = context.get_new_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[account_create_ix, mt_account_create_ix, instruction],
        Some(&context.payer.pubkey()),
        &vec![
            &context.payer,
            &new_queue_keypair,
            &new_address_merkle_tree_keypair,
        ],
        blockhash,
    );
    context
        .banks_client
        .process_transaction_with_metadata(transaction)
        .await
}

pub async fn assert_rolled_over_address_merkle_tree_and_queue(
    context: &mut ProgramTestContext,
    fee_payer_prior_balance: &u64,
    old_merkle_tree_pubkey: &Pubkey,
    old_queue_pubkey: &Pubkey,
    new_merkle_tree_pubkey: &Pubkey,
    new_queue_pubkey: &Pubkey,
) {
    let mut new_mt_account = context
        .banks_client
        .get_account(*new_merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap();
    let mut new_mt_lamports = 0u64;
    let account_info = AccountInfo::new(
        &new_merkle_tree_pubkey,
        false,
        false,
        &mut new_mt_lamports,
        &mut new_mt_account.data,
        &account_compression::ID,
        false,
        0u64,
    );
    let new_mt_account =
        AccountLoader::<AddressMerkleTreeAccount>::try_from(&account_info).unwrap();
    let new_loaded_mt_account = new_mt_account.load().unwrap();

    let mut old_mt_account = context
        .banks_client
        .get_account(*old_merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap();

    let mut old_mt_lamports = 0u64;
    let account_info = AccountInfo::new(
        &old_merkle_tree_pubkey,
        false,
        false,
        &mut old_mt_lamports,
        &mut old_mt_account.data,
        &account_compression::ID,
        false,
        0u64,
    );
    let old_mt_account =
        AccountLoader::<AddressMerkleTreeAccount>::try_from(&account_info).unwrap();
    let old_loaded_mt_account = old_mt_account.load().unwrap();
    let current_slot = context.banks_client.get_root_slot().await.unwrap();

    // Old Merkle tree
    // 1. rolled over slot is set to current slot
    // 2. next Merkle tree is set to the new Merkle tree
    assert_eq!(old_loaded_mt_account.rolledover_slot, current_slot);
    assert_eq!(
        old_loaded_mt_account.next_merkle_tree,
        *new_merkle_tree_pubkey
    );
    // New Merkle tree
    // 1. index is equal to the old Merkle tree index
    // 2. rollover fee is equal to the old Merkle tree rollover fee (the fee is calculated onchain in case rent should change the fee might be different)
    // 3. tip is equal to the old Merkle tree tip
    // 4. rollover threshold is equal to the old Merkle tree rollover threshold
    // 5. rolled over slot is set to u64::MAX (not rolled over)
    // 6. close threshold is equal to the old Merkle tree close threshold
    // 7. associated queue is equal to the new queue
    // 7. next merkle tree is set to Pubkey::default() (not set)
    // 8. owner is equal to the old Merkle tree owner
    // 9. delegate is equal to the old Merkle tree delegate

    assert_eq!(old_loaded_mt_account.index, new_loaded_mt_account.index);
    assert_eq!(
        old_loaded_mt_account.rollover_fee,
        new_loaded_mt_account.rollover_fee
    );
    assert_eq!(old_loaded_mt_account.tip, new_loaded_mt_account.tip);
    assert_eq!(
        old_loaded_mt_account.rollover_threshold,
        new_loaded_mt_account.rollover_threshold
    );
    assert_eq!(u64::MAX, new_loaded_mt_account.rolledover_slot);

    assert_eq!(
        old_loaded_mt_account.close_threshold,
        new_loaded_mt_account.close_threshold
    );
    assert_eq!(new_loaded_mt_account.associated_queue, *new_queue_pubkey);
    assert_eq!(new_loaded_mt_account.next_merkle_tree, Pubkey::default());

    assert_eq!(old_loaded_mt_account.owner, new_loaded_mt_account.owner);
    assert_eq!(
        old_loaded_mt_account.delegate,
        new_loaded_mt_account.delegate
    );

    let struct_old: *mut ConcurrentMerkleTree<Poseidon, { ADDRESS_MERKLE_TREE_HEIGHT as usize }> =
        old_loaded_mt_account.merkle_tree_struct.as_ptr() as _;
    let struct_new: *mut ConcurrentMerkleTree<Poseidon, { ADDRESS_MERKLE_TREE_HEIGHT as usize }> =
        new_loaded_mt_account.merkle_tree_struct.as_ptr() as _;
    assert_eq!(unsafe { (*struct_old).height }, unsafe {
        (*struct_new).height
    });

    assert_eq!(unsafe { (*struct_old).changelog_capacity }, unsafe {
        (*struct_new).changelog_capacity
    });

    assert_eq!(unsafe { (*struct_old).changelog_length }, unsafe {
        (*struct_new).changelog_length
    });

    assert_eq!(unsafe { (*struct_old).current_changelog_index }, unsafe {
        (*struct_new).current_changelog_index
    });

    assert_eq!(unsafe { (*struct_old).roots_capacity }, unsafe {
        (*struct_new).roots_capacity
    });

    assert_eq!(unsafe { (*struct_old).roots_length }, unsafe {
        (*struct_new).roots_length
    });

    assert_eq!(unsafe { (*struct_old).canopy_depth }, unsafe {
        (*struct_new).canopy_depth
    });

    let mut new_queue_account = context
        .banks_client
        .get_account(*new_queue_pubkey)
        .await
        .unwrap()
        .unwrap();
    let mut new_mt_lamports = 0u64;
    let account_info = AccountInfo::new(
        &new_queue_pubkey,
        false,
        false,
        &mut new_mt_lamports,
        &mut new_queue_account.data,
        &account_compression::ID,
        false,
        0u64,
    );
    let new_queue_account = AccountLoader::<AddressQueueAccount>::try_from(&account_info).unwrap();
    let new_loaded_queue_account = new_queue_account.load().unwrap();
    let mut old_queue_account = context
        .banks_client
        .get_account(*old_queue_pubkey)
        .await
        .unwrap()
        .unwrap();

    let mut old_mt_lamports = 0u64;
    let account_info = AccountInfo::new(
        &old_queue_pubkey,
        false,
        false,
        &mut old_mt_lamports,
        &mut old_queue_account.data,
        &account_compression::ID,
        false,
        0u64,
    );
    let old_queue_account = AccountLoader::<AddressQueueAccount>::try_from(&account_info).unwrap();
    let old_loaded_queue_account = old_queue_account.load().unwrap();

    assert_eq!(old_loaded_queue_account.rolledover_slot, current_slot);
    assert_eq!(
        old_loaded_queue_account.index,
        new_loaded_queue_account.index
    );
    assert_eq!(
        old_loaded_queue_account.rollover_fee,
        new_loaded_queue_account.rollover_fee
    );
    assert_eq!(old_loaded_queue_account.tip, new_loaded_queue_account.tip);
    assert_eq!(u64::MAX, new_loaded_queue_account.rolledover_slot);

    assert_eq!(
        old_loaded_queue_account.owner,
        new_loaded_queue_account.owner
    );

    assert_eq!(
        old_loaded_queue_account.delegate,
        new_loaded_queue_account.delegate
    );
    assert_eq!(
        new_loaded_queue_account.associated_merkle_tree,
        *new_merkle_tree_pubkey
    );
    assert_eq!(old_loaded_queue_account.next_queue, *new_queue_pubkey);
    assert_eq!(
        old_mt_account.get_lamports(),
        new_mt_account.get_lamports()
            + new_queue_account.get_lamports()
            + old_mt_account.get_lamports()
    );
    let fee_payer_post_balance = context
        .banks_client
        .get_account(context.payer.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;
    // rent is reimbursed, 3 signatures cost 3 x 5000 lamports
    assert_eq!(*fee_payer_prior_balance, fee_payer_post_balance + 15000);

    let old_address_queue =
        unsafe { get_hash_set::<u16, AddressQueueAccount>(context, old_queue_account.key()).await };
    let new_address_queue =
        unsafe { get_hash_set::<u16, AddressQueueAccount>(context, new_queue_account.key()).await };

    assert_eq!(
        old_address_queue.capacity_indices,
        new_address_queue.capacity_indices,
    );

    assert_eq!(
        old_address_queue.capacity_values,
        new_address_queue.capacity_values,
    );

    assert_eq!(
        old_address_queue.sequence_threshold,
        new_address_queue.sequence_threshold,
    );
}
