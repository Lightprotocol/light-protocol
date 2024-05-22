#![cfg(feature = "test-sbf")]

use num_bigint::ToBigUint;
use solana_program_test::ProgramTest;
use solana_sdk::transaction::TransactionError;
use solana_sdk::{
    instruction::InstructionError,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use thiserror::Error;

use account_compression::{
    errors::AccountCompressionErrorCode,
    utils::constants::{ADDRESS_MERKLE_TREE_CANOPY_DEPTH, ADDRESS_MERKLE_TREE_HEIGHT},
    AddressMerkleTreeConfig, AddressQueueAccount, ID,
};
use light_hash_set::HashSetError;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::{array::IndexedArray, errors::IndexedMerkleTreeError, reference};
use light_test_utils::rpc::errors::{assert_rpc_error, RpcError};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::test_rpc::ProgramTestRpcConnection;
use light_test_utils::{
    address_tree_rollover::perform_address_merkle_tree_roll_over, test_env::NOOP_PROGRAM_ID,
    test_forester::update_merkle_tree,
};
use light_test_utils::{
    address_tree_rollover::{
        assert_rolled_over_address_merkle_tree_and_queue, set_address_merkle_tree_next_index,
    },
    get_hash_set,
    test_env::create_address_merkle_tree_and_queue_account,
    test_forester::{empty_address_queue_test, insert_addresses},
    test_indexer::{AddressMerkleTreeAccounts, AddressMerkleTreeBundle},
};
use light_utils::bigint::bigint_to_be_bytes_array;

#[derive(Error, Debug)]
enum RelayerUpdateError {}

async fn relayer_update<R: RpcConnection>(
    rpc: &mut R,
    address_queue_pubkey: Pubkey,
    address_merkle_tree_pubkey: Pubkey,
) -> Result<(), RelayerUpdateError> {
    let mut relayer_indexing_array = Box::<
        IndexedArray<
            Poseidon,
            usize,
            // This is not a correct value you would normally use in relayer, A
            // correct size would be number of leaves which the merkle tree can fit
            // (`MERKLE_TREE_LEAVES`). Allocating an indexing array for over 4 mln
            // elements ain't easy and is not worth doing here.
            1000,
        >,
    >::default();
    relayer_indexing_array.init().unwrap();

    let mut relayer_merkle_tree = Box::new(
        reference::IndexedMerkleTree::<Poseidon, usize>::new(
            ADDRESS_MERKLE_TREE_HEIGHT as usize,
            ADDRESS_MERKLE_TREE_CANOPY_DEPTH as usize,
        )
        .unwrap(),
    );
    relayer_merkle_tree.init().unwrap();
    let mut address_merkle_tree_bundle = AddressMerkleTreeBundle {
        merkle_tree: relayer_merkle_tree,
        indexed_array: relayer_indexing_array,
        accounts: AddressMerkleTreeAccounts {
            merkle_tree: address_merkle_tree_pubkey,
            queue: address_queue_pubkey,
        },
    };
    empty_address_queue_test(rpc, &mut address_merkle_tree_bundle)
        .await
        .unwrap();
    Ok(())
}

/// Tests insertion of addresses to the queue, dequeuing and Merkle tree update.
#[tokio::test]
async fn test_address_queue() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    program_test.add_program("spl_noop", NOOP_PROGRAM_ID, None);
    program_test.set_compute_max_units(1_400_000u64);

    let context = program_test.start_with_context().await;
    let mut context = ProgramTestRpcConnection { context };
    let payer = context.get_payer().insecure_clone();

    let address_merkle_tree_keypair = Keypair::new();
    let address_queue_keypair = Keypair::new();
    create_address_merkle_tree_and_queue_account(
        &payer,
        &mut context,
        &address_merkle_tree_keypair,
        &address_queue_keypair,
        None,
        1,
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
        addresses.clone(),
    )
    .await
    .unwrap();
    let address_queue = unsafe {
        get_hash_set::<u16, AddressQueueAccount, ProgramTestRpcConnection>(
            &mut context,
            address_queue_keypair.pubkey(),
        )
        .await
    };

    assert!(address_queue.contains(&address1, None).unwrap());
    assert!(address_queue.contains(&address2, None).unwrap());
    relayer_update(
        &mut context,
        address_queue_keypair.pubkey(),
        address_merkle_tree_keypair.pubkey(),
    )
    .await
    .unwrap();

    let result = insert_addresses(
        &mut context,
        address_queue_keypair.pubkey(),
        address_merkle_tree_keypair.pubkey(),
        vec![bigint_to_be_bytes_array::<32>(&address1).unwrap()],
    )
    .await;
    println!("{:?}", result);
    result.unwrap_err();
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
    program_test.add_program("spl_noop", NOOP_PROGRAM_ID, None);
    let context = program_test.start_with_context().await;
    let mut context = ProgramTestRpcConnection { context };
    let payer = context.get_payer().insecure_clone();

    let address_merkle_tree_keypair = Keypair::new();
    let address_queue_keypair = Keypair::new();
    create_address_merkle_tree_and_queue_account(
        &payer,
        &mut context,
        &address_merkle_tree_keypair,
        &address_queue_keypair,
        None,
        1,
    )
    .await;

    // Local indexing array and queue. We will use them to get the correct
    // elements and Merkle proofs, which we will modify later, to pass invalid
    // values. 😈
    let mut local_indexed_array = Box::<
        IndexedArray<
            Poseidon,
            usize,
            // This is not a correct value you would normally use in relayer, A
            // correct size would be number of leaves which the merkle tree can fit
            // (`MERKLE_TREE_LEAVES`). Allocating an indexing array for over 4 mln
            // elements ain't easy and is not worth doing here.
            200,
        >,
    >::default();
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
        get_hash_set::<u16, AddressQueueAccount, ProgramTestRpcConnection>(
            &mut context,
            address_queue_keypair.pubkey(),
        )
        .await
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
        .get(low_element.next_index)
        .cloned()
        .unwrap()
        .value;
    let low_element_proof = local_merkle_tree.get_proof_of_leaf(1, false).unwrap();
    let expected_error: u32 = IndexedMerkleTreeError::LowElementGreaterOrEqualToNewElement.into();

    let error = update_merkle_tree(
        &mut context,
        address_queue_keypair.pubkey(),
        address_merkle_tree_keypair.pubkey(),
        index,
        next_index as u64,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.to_array().unwrap(),
    )
    .await
    .unwrap_err();

    // Should fail to insert the same address twice in the same tx
    assert!(matches!(
        error,
        RpcError::TransactionError(
            // ElementAlreadyExists
            TransactionError::InstructionError(0, InstructionError::Custom(error_code))
        ) if error_code == expected_error
    ));

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
        get_hash_set::<u16, AddressQueueAccount, ProgramTestRpcConnection>(
            &mut context,
            address_queue_keypair.pubkey(),
        )
        .await
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
        .get(low_element.next_index)
        .cloned()
        .unwrap()
        .value;
    let low_element_proof = local_merkle_tree.get_proof_of_leaf(0, false).unwrap();
    let expected_error: u32 = IndexedMerkleTreeError::NewElementGreaterOrEqualToNextElement.into();
    assert!(matches!(update_merkle_tree(
        &mut context,
        address_queue_keypair.pubkey(),
        address_merkle_tree_keypair.pubkey(),
        index,
        next_index as u64,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.to_array().unwrap(),
    )
    .await
    .unwrap_err(), RpcError::TransactionError(TransactionError::InstructionError(
            0,
            InstructionError::Custom(error_code)),
        ) if error_code == expected_error));
}

#[tokio::test]
async fn test_address_merkle_tree_and_queue_rollover() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    program_test.add_program("spl_noop", NOOP_PROGRAM_ID, None);
    let context = program_test.start_with_context().await;
    let mut context = ProgramTestRpcConnection { context };
    let payer = context.get_payer().insecure_clone();

    let address_merkle_tree_keypair = Keypair::new();
    let address_queue_keypair = Keypair::new();
    create_address_merkle_tree_and_queue_account(
        &payer,
        &mut context,
        &address_merkle_tree_keypair,
        &address_queue_keypair,
        None,
        1,
    )
    .await;

    let address_merkle_tree_keypair_2 = Keypair::new();
    let address_queue_keypair_2 = Keypair::new();
    create_address_merkle_tree_and_queue_account(
        &payer,
        &mut context,
        &address_merkle_tree_keypair_2,
        &address_queue_keypair_2,
        None,
        2,
    )
    .await;
    let merkle_tree_config = AddressMerkleTreeConfig::default();
    let required_next_index = 2u64.pow(26) * merkle_tree_config.rollover_threshold.unwrap() / 100;
    let failing_next_index = required_next_index - 1;

    let new_queue_keypair = Keypair::new();
    let new_address_merkle_tree_keypair = Keypair::new();

    let result = perform_address_merkle_tree_roll_over(
        &mut context,
        &new_queue_keypair,
        &new_address_merkle_tree_keypair,
        &address_merkle_tree_keypair.pubkey(),
        &address_queue_keypair.pubkey(),
    )
    .await;

    assert_rpc_error(
        result,
        2,
        AccountCompressionErrorCode::NotReadyForRollover.into(),
    );

    let lamports_queue_accounts = context
        .get_account(address_queue_keypair.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports
        + context
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
    let result = perform_address_merkle_tree_roll_over(
        &mut context,
        &new_queue_keypair,
        &new_address_merkle_tree_keypair,
        &address_merkle_tree_keypair.pubkey(),
        &address_queue_keypair.pubkey(),
    )
    .await;

    assert_rpc_error(
        result,
        2,
        AccountCompressionErrorCode::NotReadyForRollover.into(),
    );

    set_address_merkle_tree_next_index(
        &mut context,
        &address_merkle_tree_keypair.pubkey(),
        required_next_index,
        lamports_queue_accounts,
    )
    .await;

    let result = perform_address_merkle_tree_roll_over(
        &mut context,
        &new_queue_keypair,
        &new_address_merkle_tree_keypair,
        &address_merkle_tree_keypair.pubkey(),
        &address_queue_keypair_2.pubkey(),
    )
    .await;

    assert_rpc_error(
        result,
        2,
        AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated.into(),
    );

    let result = perform_address_merkle_tree_roll_over(
        &mut context,
        &new_queue_keypair,
        &new_address_merkle_tree_keypair,
        &address_merkle_tree_keypair_2.pubkey(),
        &address_queue_keypair.pubkey(),
    )
    .await;

    assert_rpc_error(
        result,
        2,
        AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated.into(),
    );

    let signer_prior_balance = context
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

    let result = perform_address_merkle_tree_roll_over(
        &mut context,
        &failing_new_nullifier_queue_keypair,
        &failing_new_state_merkle_tree_keypair,
        &address_merkle_tree_keypair.pubkey(),
        &address_queue_keypair.pubkey(),
    )
    .await;

    assert_rpc_error(
        result,
        2,
        AccountCompressionErrorCode::MerkleTreeAlreadyRolledOver.into(),
    );
}
