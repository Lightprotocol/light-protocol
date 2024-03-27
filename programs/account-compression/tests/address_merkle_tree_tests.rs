#![cfg(feature = "test-sbf")]

use std::assert_eq;

use account_compression::{
    instruction::{
        InitializeAddressMerkleTree, InitializeAddressQueue, InsertAddresses,
        UpdateAddressMerkleTree,
    },
    state::{AddressMerkleTreeAccount, AddressQueueAccount},
    utils::constants::{
        ADDRESS_MERKLE_TREE_CANOPY_DEPTH, ADDRESS_MERKLE_TREE_HEIGHT, ADDRESS_MERKLE_TREE_ROOTS,
    },
    ID,
};
use account_compression_state::address_queue_from_bytes;
use anchor_lang::InstructionData;
use ark_ff::{BigInteger, BigInteger256};
use light_hasher::Poseidon;
use light_indexed_merkle_tree::{
    array::{IndexingArray, RawIndexingElement},
    reference,
};
use light_test_utils::{create_account_instruction, AccountZeroCopy};
use light_utils::bigint::bigint_to_be_bytes;
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

fn initialize_address_queue_ix(context: &ProgramTestContext, pubkey: Pubkey) -> Instruction {
    let instruction_data = InitializeAddressQueue {};
    let initialize_ix = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(context.payer.pubkey(), true),
            AccountMeta::new(pubkey, true),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: instruction_data.data(),
    };
    initialize_ix
}

async fn create_and_initialize_address_queue(context: &mut ProgramTestContext) -> Keypair {
    let address_queue_keypair = Keypair::new();
    let account_create_ix = create_account_instruction(
        &context.payer.pubkey(),
        AddressQueueAccount::LEN,
        context
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(account_compression::AddressQueueAccount::LEN),
        &ID,
        Some(&address_queue_keypair),
    );
    // Instruction: initialize address queue.
    let initialize_ix = initialize_address_queue_ix(context, address_queue_keypair.pubkey());
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
        // TODO: check what's used since many types onchain use height 22
        height: 26,
        changelog_size: 1400,
        roots_size: 2800,
        canopy_depth: 0,
    };
    let initialize_ix = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(context.payer.pubkey(), true),
            AccountMeta::new(pubkey, true),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: instruction_data.data(),
    };
    initialize_ix
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
    addresses: Vec<[u8; 32]>,
) -> Result<(), BanksClientError> {
    let instruction_data = InsertAddresses { addresses };
    let insert_ix = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(context.payer.pubkey(), true),
            AccountMeta::new(address_queue_pubkey, false),
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

async fn update_merkle_tree(
    context: &mut ProgramTestContext,
    address_queue_pubkey: Pubkey,
    address_merkle_tree_pubkey: Pubkey,
    queue_index: u16,
    address_next_index: usize,
    address_next_value: [u8; 32],
    low_address: RawIndexingElement<usize, 32>,
    low_address_next_value: [u8; 32],
    low_address_proof: [[u8; 32]; 22],
    next_address_proof: [u8; 128],
) -> Result<(), BanksClientError> {
    let changelog_index = {
        // TODO: figure out why I get an invalid memory reference error here when I try to replace 183-190 with this
        let address_merkle_tree =
            AccountZeroCopy::<AddressMerkleTreeAccount>::new(context, address_merkle_tree_pubkey)
                .await;
        // let address_merkle_tree = context
        //     .banks_client
        //     .get_account(address_merkle_tree_pubkey)
        //     .await
        //     .unwrap()
        //     .unwrap();
        // let address_merkle_tree: &AddressMerkleTreeAccount =
        //     deserialize_account_zero_copy(&address_merkle_tree).await;

        let address_merkle_tree = &address_merkle_tree
            .deserialized()
            .load_merkle_tree()
            .unwrap();
        let changelog_index = address_merkle_tree.changelog_index();
        changelog_index
    };
    let instruction_data = UpdateAddressMerkleTree {
        changelog_index: changelog_index as u16,
        queue_index,
        address_next_index,
        address_next_value,
        low_address,
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
    let mut relayer_indexing_array = Box::new(IndexingArray::<
        Poseidon,
        usize,
        BigInteger256,
        // This is not a correct value you would normally use in relayer, A
        // correct size would be number of leaves which the merkle tree can fit
        // (`MERKLE_TREE_LEAVES`). Allocating an indexing array for over 4 mln
        // elements ain't easy and is not worth doing here.
        200,
    >::default());
    let mut relayer_merkle_tree = Box::new(
        reference::IndexedMerkleTree::<Poseidon, usize, BigInteger256>::new(
            ADDRESS_MERKLE_TREE_HEIGHT,
            ADDRESS_MERKLE_TREE_ROOTS,
            ADDRESS_MERKLE_TREE_CANOPY_DEPTH,
        )
        .unwrap(),
    );

    let mut update_errors: Vec<BanksClientError> = Vec::new();

    loop {
        let lowest_from_queue = {
            let address_queue =
                AccountZeroCopy::<AddressQueueAccount>::new(context, address_queue_pubkey).await;
            let address_queue = address_queue_from_bytes(&address_queue.deserialized().queue);
            let lowest = match address_queue.lowest() {
                Some(lowest) => lowest.clone(),
                None => break,
            };
            lowest
        };

        // Create new element from the dequeued value.
        let (old_low_address, old_low_address_next_value) = relayer_indexing_array
            .find_low_element(&lowest_from_queue.value)
            .unwrap();
        let address_bundle = relayer_indexing_array
            .new_element_with_low_element_index(old_low_address.index, lowest_from_queue.value)
            .unwrap();

        // Get the Merkle proof for updaring low element.
        let low_address_proof = relayer_merkle_tree
            .get_proof_of_leaf(usize::from(old_low_address.index), false)
            .unwrap();
        let old_low_address: RawIndexingElement<usize, 32> = old_low_address.try_into().unwrap();

        // Update on-chain tree.
        let update_successful = match update_merkle_tree(
            context,
            address_queue_pubkey,
            address_merkle_tree_pubkey,
            lowest_from_queue.index,
            address_bundle.new_element.next_index,
            bigint_to_be_bytes(&address_bundle.new_element_next_value).unwrap(),
            old_low_address,
            bigint_to_be_bytes(&old_low_address_next_value).unwrap(),
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
                    address_bundle.new_element.value,
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

/// Tests insertion of addresses to the queue, dequeuing and Merkle tree update.
#[ignore]
#[tokio::test]
async fn test_address_queue() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    let mut context = program_test.start_with_context().await;
    let address_queue_keypair = create_and_initialize_address_queue(&mut context).await;
    let address_merkle_tree_keypair = create_and_initialize_address_merkle_tree(&mut context).await;

    // Insert a pair of addresses.
    let address1 = BigInteger256::from(30_u32);
    let address2 = BigInteger256::from(10_u32);
    let addresses: Vec<[u8; 32]> = vec![
        address1.to_bytes_be().try_into().unwrap(),
        address2.to_bytes_be().try_into().unwrap(),
    ];
    insert_addresses(&mut context, address_queue_keypair.pubkey(), addresses)
        .await
        .unwrap();
    let address_queue =
        AccountZeroCopy::<AddressQueueAccount>::new(&mut context, address_queue_keypair.pubkey())
            .await;
    let address_queue = address_queue_from_bytes(&address_queue.deserialized().queue);
    let element0 = address_queue.get(0).unwrap();

    assert_eq!(element0.index, 0);
    assert_eq!(element0.value, BigInteger256::from(0_u32));
    assert_eq!(element0.next_index, 2);
    let element1 = address_queue.get(1).unwrap();
    assert_eq!(element1.index, 1);
    assert_eq!(element1.value, BigInteger256::from(30_u32));
    assert_eq!(element1.next_index, 0);
    let element2 = address_queue.get(2).unwrap();
    assert_eq!(element2.index, 2);
    assert_eq!(element2.value, BigInteger256::from(10_u32));
    assert_eq!(element2.next_index, 1);

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
#[ignore]
#[tokio::test]
async fn test_insert_invalid_low_element() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    let mut context = program_test.start_with_context().await;
    let address_queue_keypair = create_and_initialize_address_queue(&mut context).await;
    let address_merkle_tree_keypair = create_and_initialize_address_merkle_tree(&mut context).await;

    // Local indexing array and queue. We will use them to get the correct
    // elements and Merkle proofs, which we will modify later, to pass invalid
    // values. 😈
    let mut local_indexing_array = Box::new(IndexingArray::<
        Poseidon,
        usize,
        BigInteger256,
        // This is not a correct value you would normally use in relayer, A
        // correct size would be number of leaves which the merkle tree can fit
        // (`MERKLE_TREE_LEAVES`). Allocating an indexing array for over 4 mln
        // elements ain't easy and is not worth doing here.
        200,
    >::default());
    let mut local_merkle_tree = Box::new(
        reference::IndexedMerkleTree::<Poseidon, usize, BigInteger256>::new(
            ADDRESS_MERKLE_TREE_HEIGHT,
            ADDRESS_MERKLE_TREE_ROOTS,
            ADDRESS_MERKLE_TREE_CANOPY_DEPTH,
        )
        .unwrap(),
    );

    // Insert a pair of addresses, correctly. Just do it with relayer.
    let address1 = BigInteger256::from(30_u32);
    let address2 = BigInteger256::from(10_u32);
    let addresses: Vec<[u8; 32]> = vec![
        address1.to_bytes_be().try_into().unwrap(),
        address2.to_bytes_be().try_into().unwrap(),
    ];
    insert_addresses(&mut context, address_queue_keypair.pubkey(), addresses)
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
    let bundle = local_indexing_array.append(address1).unwrap();
    local_merkle_tree
        .update(
            &bundle.new_low_element,
            &bundle.new_element,
            &bundle.new_element_next_value,
        )
        .unwrap();
    let bundle = local_indexing_array.append(address2).unwrap();
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
    let address3 = BigInteger256::from(20_u32);
    let addresses: Vec<[u8; 32]> = vec![address3.to_bytes_be().try_into().unwrap()];
    insert_addresses(&mut context, address_queue_keypair.pubkey(), addresses)
        .await
        .unwrap();
    // Index of our new nullifier in the queue.
    let queue_index = 1_u16;
    // (Invalid) index of the next address.
    let next_index = 2_usize;
    // (Invalid) value of the next address.
    let next_value = address2;
    // (Invalid) low nullifier.
    let low_element = local_indexing_array.get(1).cloned().unwrap();
    let low_element_next_value = local_indexing_array
        .get(usize::from(low_element.next_index))
        .cloned()
        .unwrap()
        .value;
    let low_element_proof = local_merkle_tree.get_proof_of_leaf(1, false).unwrap();
    assert!(update_merkle_tree(
        &mut context,
        address_queue_keypair.pubkey(),
        address_merkle_tree_keypair.pubkey(),
        queue_index,
        next_index,
        bigint_to_be_bytes(&next_value).unwrap(),
        low_element.try_into().unwrap(),
        bigint_to_be_bytes(&low_element_next_value).unwrap(),
        low_element_proof.to_array().unwrap(),
        [0u8; 128],
    )
    .await
    .is_err());

    // Try inserting address 50, while pointing to index 0 as low element.
    // Therefore, the new element is greater than next element.
    let address4 = BigInteger256::from(50_u32);
    let addresses: Vec<[u8; 32]> = vec![address4.to_bytes_be().try_into().unwrap()];
    insert_addresses(&mut context, address_queue_keypair.pubkey(), addresses)
        .await
        .unwrap();
    // Index of our new nullifier in the queue.
    let queue_index = 1_u16;
    // (Invalid) index of the next address.
    let next_index = 1_usize;
    // (Invalid) value of the next address.
    let next_value = address1;
    // (Invalid) low nullifier.
    let low_element = local_indexing_array.get(0).cloned().unwrap();
    let low_element_next_value = local_indexing_array
        .get(usize::from(low_element.next_index))
        .cloned()
        .unwrap()
        .value;
    let low_element_proof = local_merkle_tree.get_proof_of_leaf(0, false).unwrap();
    assert!(update_merkle_tree(
        &mut context,
        address_queue_keypair.pubkey(),
        address_merkle_tree_keypair.pubkey(),
        queue_index,
        next_index,
        bigint_to_be_bytes(&next_value).unwrap(),
        low_element.try_into().unwrap(),
        bigint_to_be_bytes(&low_element_next_value).unwrap(),
        low_element_proof.to_array().unwrap(),
        [0u8; 128],
    )
    .await
    .is_err());
}
