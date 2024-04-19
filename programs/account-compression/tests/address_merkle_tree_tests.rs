#![cfg(feature = "test-sbf")]

use account_compression::{
    initialize_address_queue_sdk::create_initialize_address_queue_instruction,
    instruction::{InitializeAddressMerkleTree, InsertAddresses, UpdateAddressMerkleTree},
    state::AddressMerkleTreeAccount,
    utils::constants::{
        ADDRESS_MERKLE_TREE_CANOPY_DEPTH, ADDRESS_MERKLE_TREE_CHANGELOG,
        ADDRESS_MERKLE_TREE_HEIGHT, ADDRESS_MERKLE_TREE_ROOTS, ADDRESS_QUEUE_INDICES,
        ADDRESS_QUEUE_SEQUENCE_THRESHOLD, ADDRESS_QUEUE_VALUES,
    },
    AddressQueueAccount, ID,
};
use anchor_lang::InstructionData;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::{array::IndexedArray, reference};
use light_test_utils::{create_account_instruction, get_hash_set, AccountZeroCopy};
use light_utils::bigint::bigint_to_be_bytes_array;
use num_bigint::ToBigUint;
use solana_program_test::{BanksClientError, ProgramTest, ProgramTestContext};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};
use thiserror::Error;

#[derive(Error, Debug)]
enum RelayerUpdateError {
    #[error("Updating Merkle tree failed: {0:?}")]
    MerkleTreeUpdate(Vec<BanksClientError>),
}

// This is not a correct value you would normally use in relayer, A
// correct size would be number of leaves which the merkle tree can fit
// (`MERKLE_TREE_LEAVES`). Allocating an indexing array for over 4 mln
// elements ain't easy and is not worth doing here.
const INDEXED_ARRAY_ELEMENTS: usize = 200;

async fn create_and_initialize_address_queue(
    context: &mut ProgramTestContext,
    payer_pubkey: &Pubkey,
    associated_merkle_tree: Option<Pubkey>,
) -> Keypair {
    let address_queue_keypair = Keypair::new();
    let size = AddressQueueAccount::size(
        ADDRESS_QUEUE_INDICES as usize,
        ADDRESS_QUEUE_VALUES as usize,
    )
    .unwrap();
    let account_create_ix = create_account_instruction(
        &context.payer.pubkey(),
        size,
        context
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(size),
        &ID,
        Some(&address_queue_keypair),
    );
    // Instruction: initialize address queue.
    let initialize_ix = create_initialize_address_queue_instruction(
        *payer_pubkey,
        address_queue_keypair.pubkey(),
        1u64,
        associated_merkle_tree,
        ADDRESS_QUEUE_INDICES,
        ADDRESS_QUEUE_VALUES,
        ADDRESS_QUEUE_SEQUENCE_THRESHOLD,
    );
    // Transaction: initialize address queue.
    let transaction = Transaction::new_signed_with_payer(
        &[account_create_ix, initialize_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &address_queue_keypair],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
    address_queue_keypair
}

fn initialize_address_merkle_tree_ix(
    context: &ProgramTestContext,
    payer: Pubkey,
    pubkey: Pubkey,
) -> Instruction {
    let instruction_data = InitializeAddressMerkleTree {
        index: 1u64,
        owner: payer,
        delegate: None,
        height: ADDRESS_MERKLE_TREE_HEIGHT,
        changelog_size: ADDRESS_MERKLE_TREE_CHANGELOG,
        roots_size: ADDRESS_MERKLE_TREE_ROOTS,
        canopy_depth: ADDRESS_MERKLE_TREE_CANOPY_DEPTH,
    };
    Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(context.payer.pubkey(), true),
            AccountMeta::new(pubkey, true),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: instruction_data.data(),
    }
}

async fn create_and_initialize_address_merkle_tree(context: &mut ProgramTestContext) -> Keypair {
    let address_merkle_tree_keypair = Keypair::new();
    let account_create_ix = create_account_instruction(
        &context.payer.pubkey(),
        AddressMerkleTreeAccount::LEN,
        context
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(account_compression::AddressMerkleTreeAccount::LEN),
        &ID,
        Some(&address_merkle_tree_keypair),
    );
    // Instruction: initialize address Merkle tree.
    let initialize_ix = initialize_address_merkle_tree_ix(
        context,
        context.payer.pubkey(),
        address_merkle_tree_keypair.pubkey(),
    );
    // Transaction: initialize address Merkle tree.
    let transaction = Transaction::new_signed_with_payer(
        &[account_create_ix, initialize_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &address_merkle_tree_keypair],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
    address_merkle_tree_keypair
}

async fn insert_addresses(
    context: &mut ProgramTestContext,
    address_queue_pubkey: Pubkey,
    address_merkle_tree_pubkey: Pubkey,
    addresses: Vec<[u8; 32]>,
) -> Result<(), BanksClientError> {
    let instruction_data = InsertAddresses { addresses };
    let insert_ix = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(context.payer.pubkey(), true),
            AccountMeta::new(address_queue_pubkey, false),
            AccountMeta::new(address_merkle_tree_pubkey, false),
        ],
        data: instruction_data.data(),
    };
    let transaction = Transaction::new_signed_with_payer(
        &[insert_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(transaction).await
}

#[allow(clippy::too_many_arguments)]
async fn update_merkle_tree(
    context: &mut ProgramTestContext,
    address_queue_pubkey: Pubkey,
    address_merkle_tree_pubkey: Pubkey,
    value: [u8; 32],
    next_index: u64,
    next_value: [u8; 32],
    low_address_index: u64,
    low_address_value: [u8; 32],
    low_address_next_index: u64,
    low_address_next_value: [u8; 32],
    low_address_proof: [[u8; 32]; 16],
    next_address_proof: [u8; 128],
) -> Result<(), BanksClientError> {
    let changelog_index = {
        // TODO: figure out why I get an invalid memory reference error here when I try to replace 183-190 with this
        let address_merkle_tree =
            AccountZeroCopy::<AddressMerkleTreeAccount>::new(context, address_merkle_tree_pubkey)
                .await;

        let address_merkle_tree = &address_merkle_tree
            .deserialized()
            .load_merkle_tree()
            .unwrap();
        address_merkle_tree.changelog_index()
    };
    let instruction_data = UpdateAddressMerkleTree {
        changelog_index: changelog_index as u16,
        value,
        next_index,
        next_value,
        low_address_index,
        low_address_value,
        low_address_next_index,
        low_address_next_value,
        low_address_proof,
        next_address_proof,
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
    context.banks_client.process_transaction(transaction).await
}

async fn relayer_update(
    context: &mut ProgramTestContext,
    address_queue_pubkey: Pubkey,
    address_merkle_tree_pubkey: Pubkey,
) -> Result<(), RelayerUpdateError> {
    let mut relayer_indexing_array =
        Box::<IndexedArray<Poseidon, usize, INDEXED_ARRAY_ELEMENTS>>::default();
    let mut relayer_merkle_tree = Box::new(
        reference::IndexedMerkleTree::<Poseidon, usize>::new(
            ADDRESS_MERKLE_TREE_HEIGHT as usize,
            ADDRESS_MERKLE_TREE_CANOPY_DEPTH as usize,
        )
        .unwrap(),
    );

    let mut update_errors: Vec<BanksClientError> = Vec::new();

    let address_merkle_tree =
        AccountZeroCopy::<AddressMerkleTreeAccount>::new(context, address_merkle_tree_pubkey).await;
    let address_merkle_tree = &address_merkle_tree
        .deserialized()
        .load_merkle_tree()
        .unwrap();

    let address_queue =
        unsafe { get_hash_set::<u16, AddressQueueAccount>(context, address_queue_pubkey).await };

    loop {
        let address = address_queue
            .first(address_merkle_tree.merkle_tree.sequence_number)
            .unwrap();
        if address.is_none() {
            break;
        }
        let address = address.unwrap();

        // Create new element from the dequeued value.
        let (old_low_address, old_low_address_next_value) = relayer_indexing_array
            .find_low_element(&address.value_biguint())
            .unwrap();
        let address_bundle = relayer_indexing_array
            .new_element_with_low_element_index(old_low_address.index, &address.value_biguint())
            .unwrap();

        // Get the Merkle proof for updating low element.
        let low_address_proof = relayer_merkle_tree
            .get_proof_of_leaf(old_low_address.index, false)
            .unwrap();

        // Update on-chain tree.
        let update_successful = match update_merkle_tree(
            context,
            address_queue_pubkey,
            address_merkle_tree_pubkey,
            bigint_to_be_bytes_array(&address.value_biguint()).unwrap(),
            address_bundle.new_element.next_index as u64,
            bigint_to_be_bytes_array(&address_bundle.new_element_next_value).unwrap(),
            old_low_address.index as u64,
            bigint_to_be_bytes_array(&old_low_address.value).unwrap(),
            old_low_address.next_index as u64,
            bigint_to_be_bytes_array(&old_low_address_next_value).unwrap(),
            low_address_proof.to_array().unwrap(),
            [0u8; 128],
        )
        .await
        {
            Ok(_) => true,
            Err(e) => {
                update_errors.push(e);
                false
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
        Err(RelayerUpdateError::MerkleTreeUpdate(update_errors))
    }
}

// TODO: enable address Merkle tree tests
/// Tests insertion of addresses to the queue, de-queuing and Merkle tree update.
#[tokio::test]
#[ignore]
async fn test_address_queue() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    program_test.set_compute_max_units(1_400_000u64);

    let mut context = program_test.start_with_context().await;

    let payer = context.payer.pubkey();

    let address_merkle_tree_keypair = create_and_initialize_address_merkle_tree(&mut context).await;
    let address_queue_keypair = create_and_initialize_address_queue(
        &mut context,
        &payer,
        Some(address_merkle_tree_keypair.pubkey()),
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
        .load_merkle_tree()
        .unwrap();

    let address_queue = unsafe {
        get_hash_set::<u16, AddressQueueAccount>(&mut context, address_queue_keypair.pubkey()).await
    };

    assert!(address_queue
        .contains(&address1, address_merkle_tree.merkle_tree.sequence_number)
        .unwrap());
    assert!(address_queue
        .contains(&address2, address_merkle_tree.merkle_tree.sequence_number)
        .unwrap());

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
#[ignore]
async fn test_insert_invalid_low_element() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    let mut context = program_test.start_with_context().await;

    let payer = context.payer.pubkey();

    let address_merkle_tree_keypair = create_and_initialize_address_merkle_tree(&mut context).await;
    let address_queue_keypair = create_and_initialize_address_queue(
        &mut context,
        &payer,
        Some(address_merkle_tree_keypair.pubkey()),
    )
    .await;

    // Local indexing array and queue. We will use them to get the correct
    // elements and Merkle proofs, which we will modify later, to pass invalid
    // values. 😈

    let mut local_indexed_array =
        Box::<IndexedArray<Poseidon, usize, INDEXED_ARRAY_ELEMENTS>>::default();
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
    // (Invalid) index of the next address.
    let next_index = 2_usize;
    // (Invalid) value of the next address.
    let next_value = address2;
    // (Invalid) low nullifier.
    let low_element = local_indexed_array.get(1).cloned().unwrap();
    let low_element_next_value = local_indexed_array
        .get(low_element.next_index)
        .cloned()
        .unwrap()
        .value;
    let low_element_proof = local_merkle_tree.get_proof_of_leaf(1, false).unwrap();
    assert!(update_merkle_tree(
        &mut context,
        address_queue_keypair.pubkey(),
        address_merkle_tree_keypair.pubkey(),
        bigint_to_be_bytes_array(&address3).unwrap(),
        next_index as u64,
        bigint_to_be_bytes_array(&next_value).unwrap(),
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.to_array().unwrap(),
        [0u8; 128],
    )
    .await
    .is_err());

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
    // Index of our new nullifier in the queue.
    // let queue_index = 1_u16;
    // (Invalid) index of the next address.
    let next_index = 1_usize;
    // (Invalid) value of the next address.
    let next_value = address1;
    // (Invalid) low nullifier.
    let low_element = local_indexed_array.get(0).cloned().unwrap();
    let low_element_next_value = local_indexed_array
        .get(low_element.next_index)
        .cloned()
        .unwrap()
        .value;
    let low_element_proof = local_merkle_tree.get_proof_of_leaf(0, false).unwrap();
    assert!(update_merkle_tree(
        &mut context,
        address_queue_keypair.pubkey(),
        address_merkle_tree_keypair.pubkey(),
        bigint_to_be_bytes_array(&address4).unwrap(),
        next_index as u64,
        bigint_to_be_bytes_array(&next_value).unwrap(),
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.to_array().unwrap(),
        [0u8; 128],
    )
    .await
    .is_err());
}
