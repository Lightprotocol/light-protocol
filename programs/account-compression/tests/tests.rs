#![cfg(feature = "test-sbf")]

use account_compression::{
    instruction::{
        InitializeAddressMerkleTree, InitializeAddressQueue, InsertAddresses,
        UpdateAddressMerkleTree,
    },
    state::{AddressMerkleTreeAccount, AddressQueueAccount},
    ID,
};
use account_compression_state::{
    address_merkle_tree_from_bytes, address_queue_from_bytes, MERKLE_TREE_HEIGHT,
    MERKLE_TREE_ROOTS, QUEUE_ELEMENTS,
};
use anchor_lang::{InstructionData, ZeroCopy};
use ark_ff::{BigInteger, BigInteger256};
use light_hasher::Poseidon;
use light_indexed_merkle_tree::{
    array::{IndexingArray, RawIndexingElement},
    reference,
};
use light_utils::bigint_to_be_bytes;
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction, system_program,
    transaction::Transaction,
};

// async fn get_account_zero_copy<T>(context: &mut ProgramTestContext, pubkey: Pubkey) -> Box<T>
async fn deserialize_account_zero_copy<'a, T>(account: &'a Account) -> &'a T
where
    T: ZeroCopy,
{
    // TODO: Check discriminator.
    unsafe {
        let ptr = account.data[8..].as_ptr() as *const T;
        &*ptr
    }
}

async fn create_account_ix(
    context: &mut ProgramTestContext,
    size: usize,
) -> (Keypair, Instruction) {
    let keypair = Keypair::new();
    let instruction = system_instruction::create_account(
        &context.payer.pubkey(),
        &keypair.pubkey(),
        context
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(size),
        size as u64,
        &ID,
    );
    (keypair, instruction)
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
    let (address_queue_keypair, account_create_ix) =
        create_account_ix(context, AddressQueueAccount::LEN).await;
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

fn initialize_address_merkle_tree_ix(context: &ProgramTestContext, pubkey: Pubkey) -> Instruction {
    let instruction_data = InitializeAddressMerkleTree {};
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
    let (address_merkle_tree_keypair, account_create_ix) =
        create_account_ix(context, AddressMerkleTreeAccount::LEN).await;
    // Instruction: initialize address Merkle tree.
    let initialize_ix =
        initialize_address_merkle_tree_ix(context, address_merkle_tree_keypair.pubkey());
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
) {
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
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

async fn update_merkle_tree(
    context: &mut ProgramTestContext,
    address_queue_pubkey: Pubkey,
    address_merkle_tree_pubkey: Pubkey,
    queue_index: u16,
    // address_index: u16,
    address_next_index: u16,
    address_next_value: [u8; 32],
    low_address: RawIndexingElement<32>,
    low_address_next_value: [u8; 32],
    low_address_proof: [[u8; 32]; 22],
    next_address_proof: [u8; 128],
) {
    let changelog_index = {
        let address_merkle_tree = context
            .banks_client
            .get_account(address_merkle_tree_pubkey)
            .await
            .unwrap()
            .unwrap();
        let address_merkle_tree: &AddressMerkleTreeAccount =
            deserialize_account_zero_copy(&address_merkle_tree).await;
        let address_merkle_tree = address_merkle_tree_from_bytes(&address_merkle_tree.merkle_tree);
        let changelog_index = address_merkle_tree.changelog_index();
        changelog_index
    };
    let instruction_data = UpdateAddressMerkleTree {
        changelog_index: changelog_index as u16,
        queue_index,
        // address_index,
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
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

async fn relayer_update(
    context: &mut ProgramTestContext,
    address_queue_pubkey: Pubkey,
    address_merkle_tree_pubkey: Pubkey,
) {
    let mut relayer_indexing_array =
        IndexingArray::<Poseidon, BigInteger256, QUEUE_ELEMENTS>::default();
    let mut relayer_merkle_tree = reference::IndexedMerkleTree::<
        Poseidon,
        BigInteger256,
        MERKLE_TREE_HEIGHT,
        MERKLE_TREE_ROOTS,
    >::new()
    .unwrap();
    loop {
        let lowest_from_queue = {
            let address_queue = context
                .banks_client
                .get_account(address_queue_pubkey)
                .await
                .unwrap()
                .unwrap();
            let address_queue: &AddressQueueAccount =
                deserialize_account_zero_copy(&address_queue).await;
            let address_queue = address_queue_from_bytes(&address_queue.queue);
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
            .new_element_with_low_element_index(old_low_address.index, lowest_from_queue.value);

        // Get the Merkle proof for updaring low element.
        let low_address_proof =
            relayer_merkle_tree.get_proof_of_leaf(usize::from(old_low_address.index));
        let old_low_address: RawIndexingElement<32> = old_low_address.try_into().unwrap();

        update_merkle_tree(
            context,
            address_queue_pubkey,
            address_merkle_tree_pubkey,
            lowest_from_queue.index,
            address_bundle.new_element.next_index,
            bigint_to_be_bytes(&address_bundle.new_element_next_value).unwrap(),
            old_low_address,
            bigint_to_be_bytes(&old_low_address_next_value).unwrap(),
            low_address_proof,
            [0u8; 128],
        )
        .await;

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

/// Test insertion of addresses to the queue, dequeuing and Merkle tree update.
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
    insert_addresses(&mut context, address_queue_keypair.pubkey(), addresses).await;

    // Check if addresses were inserted properly.
    let address_queue = context
        .banks_client
        .get_account(address_queue_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let address_queue: &AddressQueueAccount = deserialize_account_zero_copy(&address_queue).await;
    let address_queue = address_queue_from_bytes(&address_queue.queue);
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
    .await;
}
