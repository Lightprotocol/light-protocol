#![cfg(feature = "test-sbf")]

use account_compression::{
    errors::AccountCompressionErrorCode,
    state::QueueAccount,
    utils::constants::{ADDRESS_MERKLE_TREE_CANOPY_DEPTH, ADDRESS_MERKLE_TREE_HEIGHT},
    AddressMerkleTreeAccount, AddressMerkleTreeConfig, ID,
};
use anchor_lang::error::ErrorCode;
use light_hash_set::{HashSet, HashSetError};
use light_hasher::Poseidon;
use light_indexed_merkle_tree::{array::IndexedArray, errors::IndexedMerkleTreeError, reference};
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
    indexer::{AddressMerkleTreeAccounts, AddressMerkleTreeBundle},
    test_env::create_address_merkle_tree_and_queue_account,
    test_forester::{empty_address_queue_test, insert_addresses},
};
use light_test_utils::{airdrop_lamports, rpc::rpc_connection::RpcConnection};
use light_test_utils::{rpc::errors::assert_rpc_error, AccountZeroCopy};
use light_utils::bigint::bigint_to_be_bytes_array;
use num_bigint::ToBigUint;
use solana_program_test::ProgramTest;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

/// Tests insertion of addresses to the queue, dequeuing and Merkle tree update.
/// 1. create address Merkle tree and queue accounts
/// 2. inserts two addresses to the queue
/// 3. inserts two addresses into the address Merkle tree
/// 4. insert third address
#[tokio::test]
async fn test_address_queue_and_tree_functional() {
    // CHECK: 1 create address Merkle tree and queue accounts
    let (mut context, _, mut address_merkle_tree_bundle) =
        test_setup_with_address_merkle_tree().await;
    let address_queue_pubkey = address_merkle_tree_bundle.accounts.queue;
    let address_merkle_tree_pubkey = address_merkle_tree_bundle.accounts.merkle_tree;
    // Insert a pair of addresses.
    let address1 = 30_u32.to_biguint().unwrap();
    let address2 = 10_u32.to_biguint().unwrap();
    let addresses: Vec<[u8; 32]> = vec![
        bigint_to_be_bytes_array(&address1).unwrap(),
        bigint_to_be_bytes_array(&address2).unwrap(),
    ];
    // CHECK: 2 inserts two addresses to the queue
    insert_addresses(
        &mut context,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        addresses.clone(),
    )
    .await
    .unwrap();
    let address_queue = unsafe {
        get_hash_set::<QueueAccount, ProgramTestRpcConnection>(&mut context, address_queue_pubkey)
            .await
    };

    assert!(address_queue.contains(&address1, None).unwrap());
    assert!(address_queue.contains(&address2, None).unwrap());

    // CHECK: 3 inserts two addresses into the address Merkle tree
    empty_address_queue_test(&mut context, &mut address_merkle_tree_bundle, None)
        .await
        .unwrap();

    let address3 = 20_u32.to_biguint().unwrap();
    let addresses: Vec<[u8; 32]> = vec![bigint_to_be_bytes_array(&address3).unwrap()];
    insert_addresses(
        &mut context,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        addresses,
    )
    .await
    .unwrap();
    let address_queue = unsafe {
        get_hash_set::<QueueAccount, ProgramTestRpcConnection>(&mut context, address_queue_pubkey)
            .await
    };
    address_queue
        .find_element(&address3, None)
        .unwrap()
        .unwrap();
    // CHECK: 4 insert third address which is inbetween the first two addresses
    empty_address_queue_test(&mut context, &mut address_merkle_tree_bundle, None)
        .await
        .unwrap();
}

/// Try to insert an address to the tree while pointing to an invalid low
/// address.
///
/// Such invalid insertion needs to be performed manually, without relayer's
/// help (which would always insert that nullifier correctly).
/// Tests:
/// 1. cannot insert the same address twice
/// 2. cannot insert an address with an invalid low address
/// 2.1 cannot insert an address with an invalid low address (NewElementGreaterOrEqualToNextElement)
/// 2.2 cannot insert an address with an invalid low address (LowElementGreaterOrEqualToNewElement)
/// 3.1 invalid value index (element does not exist)
/// 3.2 invalid value index (element has a sequence number)
/// 4. invalid low element index
/// 5. invalid low element value
/// 6. invalid low element next index
/// 7. invalid low element next value
/// 8. invalid low element proof
/// 9. invalid changelog index (lower)
/// 10. invalid changelog index (higher)
/// 11. invalid queue account
/// 12. invalid Merkle tree account
/// 13. non-associated Merkle tree
#[tokio::test]
async fn update_address_merkle_tree_failing_tests() {
    let (mut context, payer, mut address_merkle_tree_bundle) =
        test_setup_with_address_merkle_tree().await;
    let address_queue_pubkey = address_merkle_tree_bundle.accounts.queue;
    let address_merkle_tree_pubkey = address_merkle_tree_bundle.accounts.merkle_tree;
    // Insert a pair of addresses, correctly. Just do it with relayer.
    let address1 = 30_u32.to_biguint().unwrap();
    let address2 = 10_u32.to_biguint().unwrap();
    let addresses: Vec<[u8; 32]> = vec![
        bigint_to_be_bytes_array(&address1).unwrap(),
        bigint_to_be_bytes_array(&address2).unwrap(),
    ];

    insert_addresses(
        &mut context,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        addresses,
    )
    .await
    .unwrap();
    empty_address_queue_test(&mut context, &mut address_merkle_tree_bundle, None)
        .await
        .unwrap();
    // CHECK: 1 cannot insert the same address twice
    let result = insert_addresses(
        &mut context,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        vec![bigint_to_be_bytes_array::<32>(&address1).unwrap()],
    )
    .await;
    assert_rpc_error(result, 0, HashSetError::ElementAlreadyExists.into()).unwrap();
    let result = insert_addresses(
        &mut context,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        vec![bigint_to_be_bytes_array::<32>(&address2).unwrap()],
    )
    .await;
    assert_rpc_error(result, 0, HashSetError::ElementAlreadyExists.into()).unwrap();

    // Insert address3=20 for subsequent failing tests.
    let address3 = 20_u32.to_biguint().unwrap();
    let address3_bytes = bigint_to_be_bytes_array::<32>(&address3).unwrap();
    insert_addresses(
        &mut context,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        vec![address3_bytes],
    )
    .await
    .unwrap();
    let address4 = 21_u32.to_biguint().unwrap();
    let address4_bytes = bigint_to_be_bytes_array::<32>(&address4).unwrap();
    insert_addresses(
        &mut context,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        vec![address4_bytes],
    )
    .await
    .unwrap();
    let address_queue = unsafe {
        get_hash_set::<QueueAccount, ProgramTestRpcConnection>(&mut context, address_queue_pubkey)
            .await
    };
    // CHECK: 2.1 cannot insert an address with an invalid low address
    test_with_invalid_low_element(
        &mut context,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        &address_queue,
        &address_merkle_tree_bundle,
        0,
        IndexedMerkleTreeError::NewElementGreaterOrEqualToNextElement.into(),
    )
    .await;
    // CHECK: 2.2 cannot insert an address with an invalid low address
    test_with_invalid_low_element(
        &mut context,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        &address_queue,
        &address_merkle_tree_bundle,
        1,
        IndexedMerkleTreeError::LowElementGreaterOrEqualToNewElement.into(),
    )
    .await;

    let (address, address_hashset_index) = address_queue.first_no_seq().unwrap().unwrap();
    let (low_element, low_element_next_value) = address_merkle_tree_bundle
        .indexed_array
        .find_low_element_for_nonexistent(&address.value_biguint())
        .unwrap();
    // Get the Merkle proof for updating low element.
    let low_element_proof = address_merkle_tree_bundle
        .merkle_tree
        .get_proof_of_leaf(low_element.index, false)
        .unwrap();
    let value_index = address_hashset_index;

    // CHECK: 3.1 invalid value index (value doesn't exist)
    let invalid_value_index = 10;
    // unwraps on a None value onchain.
    update_merkle_tree(
        &mut context,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        invalid_value_index,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.to_array().unwrap(),
        None,
        None,
    )
    .await
    .unwrap_err();
    // CHECK: 3.2 invalid value index (value has a sequence number)
    let invalid_value_index = 0;
    // unwraps on a None value onchain.
    update_merkle_tree(
        &mut context,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        invalid_value_index,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.to_array().unwrap(),
        None,
        None,
    )
    .await
    .unwrap_err();
    // CHECK: 4 invalid low element index
    let invalid_lower_element_index = low_element.index - 1;
    let error_invalid_low_element_index = update_merkle_tree(
        &mut context,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        value_index,
        invalid_lower_element_index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.to_array().unwrap(),
        None,
        None,
    )
    .await
    .unwrap_err();
    assert_rpc_error(
        Err(error_invalid_low_element_index),
        0,
        10008, // ConcurrentMerkleTreeError::InvalidProof
    )
    .unwrap();

    // CHECK: 5 invalid low element value
    let invalid_low_element_value = [0u8; 32];
    let error_invalid_low_element_value = update_merkle_tree(
        &mut context,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        value_index,
        low_element.index as u64,
        invalid_low_element_value,
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.to_array().unwrap(),
        None,
        None,
    )
    .await
    .unwrap_err();
    assert_rpc_error(
        Err(error_invalid_low_element_value),
        0,
        10008, // ConcurrentMerkleTreeError::InvalidProof
    )
    .unwrap();

    // CHECK: 6 invalid low element next index
    let invalid_low_element_next_index = 1;
    let error_invalid_low_element_next_index = update_merkle_tree(
        &mut context,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        value_index,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        invalid_low_element_next_index,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.to_array().unwrap(),
        None,
        None,
    )
    .await
    .unwrap_err();
    assert_rpc_error(
        Err(error_invalid_low_element_next_index),
        0,
        10008, // ConcurrentMerkleTreeError::InvalidProof
    )
    .unwrap();

    // CHECK: 7 invalid low element next value
    let invalid_low_element_next_value = [9u8; 32];
    let error_invalid_low_element_next_value = update_merkle_tree(
        &mut context,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        value_index,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        invalid_low_element_next_value,
        low_element_proof.to_array().unwrap(),
        None,
        None,
    )
    .await
    .unwrap_err();
    assert_rpc_error(
        Err(error_invalid_low_element_next_value),
        0,
        10008, // ConcurrentMerkleTreeError::InvalidProof
    )
    .unwrap();

    // CHECK: 8 invalid low element proof
    let mut invalid_low_element_proof = low_element_proof.to_array().unwrap();
    invalid_low_element_proof.get_mut(0).unwrap()[0] = 0;
    let error_invalid_low_element_proof = update_merkle_tree(
        &mut context,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        value_index,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        invalid_low_element_proof,
        None,
        None,
    )
    .await
    .unwrap_err();
    assert_rpc_error(
        Err(error_invalid_low_element_proof),
        0,
        10008, // ConcurrentMerkleTreeError::InvalidProof
    )
    .unwrap();
    let address_merkle_tree =
        AccountZeroCopy::<AddressMerkleTreeAccount>::new(&mut context, address_merkle_tree_pubkey)
            .await;
    let address_merkle_tree = &address_merkle_tree
        .deserialized()
        .load_merkle_tree()
        .unwrap();
    let changelog_index = address_merkle_tree.merkle_tree.changelog_index();
    // CHECK: 9 invalid changelog index
    let invalid_changelog_index_low = changelog_index - 2;
    let error_invalid_changelog_index_low = update_merkle_tree(
        &mut context,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        value_index,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.to_array().unwrap(),
        Some(invalid_changelog_index_low as u16),
        None,
    )
    .await
    .unwrap_err();
    assert_rpc_error(
        Err(error_invalid_changelog_index_low),
        0,
        10009, // ConcurrentMerkleTreeError::InvalidProof
    )
    .unwrap();

    let invalid_changelog_index_high = changelog_index + 2;
    let error_invalid_changelog_index_high = update_merkle_tree(
        &mut context,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        value_index,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.to_array().unwrap(),
        Some(invalid_changelog_index_high as u16),
        None,
    )
    .await
    .unwrap_err();
    assert_rpc_error(
        Err(error_invalid_changelog_index_high),
        0,
        8003, // BoundedVecError::IterFromOutOfBounds
    )
    .unwrap();
    // CHECK: 11 invalid queue account
    let invalid_queue = address_merkle_tree_pubkey;
    let error_invalid_queue = update_merkle_tree(
        &mut context,
        invalid_queue,
        address_merkle_tree_pubkey,
        value_index,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.to_array().unwrap(),
        None,
        None,
    )
    .await
    .unwrap_err();
    assert_rpc_error(
        Err(error_invalid_queue),
        0,
        ErrorCode::AccountDiscriminatorMismatch.into(),
    )
    .unwrap();

    // CHECK: 12 invalid Merkle tree account
    let invalid_merkle_tree = address_queue_pubkey;
    let error_invalid_merkle_tree = update_merkle_tree(
        &mut context,
        address_queue_pubkey,
        invalid_merkle_tree,
        value_index,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.to_array().unwrap(),
        Some(changelog_index as u16),
        None,
    )
    .await
    .unwrap_err();
    assert_rpc_error(
        Err(error_invalid_merkle_tree),
        0,
        ErrorCode::AccountDiscriminatorMismatch.into(),
    )
    .unwrap();

    let invalid_address_merkle_tree_keypair = Keypair::new();
    let invalid_address_queue_keypair = Keypair::new();
    create_address_merkle_tree_and_queue_account(
        &payer,
        &payer.pubkey(),
        &mut context,
        &invalid_address_merkle_tree_keypair,
        &invalid_address_queue_keypair,
        None,
        2,
    )
    .await;

    // CHECK: 13 non-associated Merkle tree
    let invalid_merkle_tree = invalid_address_merkle_tree_keypair.pubkey();
    let error_non_associated_merkle_tree = update_merkle_tree(
        &mut context,
        address_queue_pubkey,
        invalid_merkle_tree,
        value_index,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.to_array().unwrap(),
        Some(changelog_index as u16),
        None,
    )
    .await
    .unwrap_err();
    assert_rpc_error(
        Err(error_non_associated_merkle_tree),
        0,
        AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated.into(),
    )
    .unwrap();
}

/// Tests address Merkle tree and queue rollover.
/// 1. Not ready for rollover after init.
/// 2. Not ready for rollover after setting next index to required value - 1.
/// 3. Merkle tree and queue not associated (Invalid queue).
/// 4. Merkle tree and queue not associated (Invalid Merkle tree).
/// 5. Successful rollover.
/// 6. Attempt to rollover already rolled over Queue and Merkle tree.
#[tokio::test]
async fn test_address_merkle_tree_and_queue_rollover() {
    let (mut context, payer, bundle) = test_setup_with_address_merkle_tree().await;
    let address_merkle_tree_pubkey = bundle.accounts.merkle_tree;
    let address_queue_pubkey = bundle.accounts.queue;
    let address_merkle_tree_keypair_2 = Keypair::new();
    let address_queue_keypair_2 = Keypair::new();
    create_address_merkle_tree_and_queue_account(
        &payer,
        &payer.pubkey(),
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

    // CHECK 1: Not ready for rollover after init.
    let result = perform_address_merkle_tree_roll_over(
        &mut context,
        &new_queue_keypair,
        &new_address_merkle_tree_keypair,
        &address_merkle_tree_pubkey,
        &address_queue_pubkey,
    )
    .await;

    assert_rpc_error(
        result,
        2,
        AccountCompressionErrorCode::NotReadyForRollover.into(),
    )
    .unwrap();

    let rollover_costs = context
        .get_account(address_queue_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports
        + context
            .get_account(address_merkle_tree_pubkey)
            .await
            .unwrap()
            .unwrap()
            .lamports;
    // Airdrop sufficient funds to address queue to reimburse the rollover costs.
    airdrop_lamports(&mut context, &address_queue_pubkey, rollover_costs)
        .await
        .unwrap();
    let address_merkle_tree_lamports = context
        .get_account(address_merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    set_address_merkle_tree_next_index(
        &mut context,
        &address_merkle_tree_pubkey,
        failing_next_index,
        address_merkle_tree_lamports,
    )
    .await;

    // CHECK 2: Not ready for rollover after setting next index to required value - 1.
    let result = perform_address_merkle_tree_roll_over(
        &mut context,
        &new_queue_keypair,
        &new_address_merkle_tree_keypair,
        &address_merkle_tree_pubkey,
        &address_queue_pubkey,
    )
    .await;

    assert_rpc_error(
        result,
        2,
        AccountCompressionErrorCode::NotReadyForRollover.into(),
    )
    .unwrap();

    set_address_merkle_tree_next_index(
        &mut context,
        &address_merkle_tree_pubkey,
        required_next_index,
        address_merkle_tree_lamports,
    )
    .await;

    // CHECK 3: Merkle tree and queue not associated invalid queue.
    let result = perform_address_merkle_tree_roll_over(
        &mut context,
        &new_queue_keypair,
        &new_address_merkle_tree_keypair,
        &address_merkle_tree_pubkey,
        &address_queue_keypair_2.pubkey(),
    )
    .await;

    assert_rpc_error(
        result,
        2,
        AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated.into(),
    )
    .unwrap();

    // CHECK 4: Merkle tree and queue not associated invalid Merkle tree.
    let result = perform_address_merkle_tree_roll_over(
        &mut context,
        &new_queue_keypair,
        &new_address_merkle_tree_keypair,
        &address_merkle_tree_keypair_2.pubkey(),
        &address_queue_pubkey,
    )
    .await;

    assert_rpc_error(
        result,
        2,
        AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated.into(),
    )
    .unwrap();

    let signer_prior_balance = context
        .get_account(payer.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;
    // CHECK 5: Successful rollover.
    perform_address_merkle_tree_roll_over(
        &mut context,
        &new_queue_keypair,
        &new_address_merkle_tree_keypair,
        &address_merkle_tree_pubkey,
        &address_queue_pubkey,
    )
    .await
    .unwrap();

    assert_rolled_over_address_merkle_tree_and_queue(
        &mut context,
        &signer_prior_balance,
        &address_merkle_tree_pubkey,
        &address_queue_pubkey,
        &new_address_merkle_tree_keypair.pubkey(),
        &new_queue_keypair.pubkey(),
    )
    .await;

    let failing_new_nullifier_queue_keypair = Keypair::new();
    let failing_new_state_merkle_tree_keypair = Keypair::new();

    // CHECK 6: Attempt to rollover already rolled over Queue and Merkle tree.
    let result = perform_address_merkle_tree_roll_over(
        &mut context,
        &failing_new_nullifier_queue_keypair,
        &failing_new_state_merkle_tree_keypair,
        &address_merkle_tree_pubkey,
        &address_queue_pubkey,
    )
    .await;

    assert_rpc_error(
        result,
        2,
        AccountCompressionErrorCode::MerkleTreeAlreadyRolledOver.into(),
    )
    .unwrap();
}

pub async fn test_setup_with_address_merkle_tree() -> (
    ProgramTestRpcConnection, // rpc
    Keypair,                  // payer
    AddressMerkleTreeBundle<200>,
) {
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
        &payer.pubkey(),
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
    local_indexed_array.init().unwrap();

    let mut local_merkle_tree = Box::new(
        reference::IndexedMerkleTree::<Poseidon, usize>::new(
            ADDRESS_MERKLE_TREE_HEIGHT as usize,
            ADDRESS_MERKLE_TREE_CANOPY_DEPTH as usize,
        )
        .unwrap(),
    );
    local_merkle_tree.init().unwrap();
    let address_merkle_tree_bundle = AddressMerkleTreeBundle::<200> {
        merkle_tree: local_merkle_tree,
        indexed_array: local_indexed_array,
        accounts: AddressMerkleTreeAccounts {
            merkle_tree: address_merkle_tree_keypair.pubkey(),
            queue: address_queue_keypair.pubkey(),
        },
    };
    (context, payer, address_merkle_tree_bundle)
}

pub async fn test_with_invalid_low_element(
    context: &mut ProgramTestRpcConnection,
    address_queue_pubkey: Pubkey,
    address_merkle_tree_pubkey: Pubkey,
    address_queue: &HashSet,
    address_merkle_tree_bundle: &AddressMerkleTreeBundle<200>,
    index: usize,
    expected_error: u32,
) {
    let (_, address_hashset_index) = address_queue.first_no_seq().unwrap().unwrap();
    let low_element = address_merkle_tree_bundle.indexed_array.get(index).unwrap();
    let low_element_next_value = address_merkle_tree_bundle
        .indexed_array
        .get(low_element.next_index())
        .unwrap()
        .value
        .clone();
    // Get the Merkle proof for updating low element.
    let low_element_proof = address_merkle_tree_bundle
        .merkle_tree
        .get_proof_of_leaf(low_element.index, false)
        .unwrap();
    let value_index = address_hashset_index;

    // unwraps on a None value onchain.
    let error_invalid_low_element = update_merkle_tree(
        context,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        value_index,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.to_array().unwrap(),
        None,
        None,
    )
    .await
    .unwrap_err();
    assert_rpc_error(Err(error_invalid_low_element), 0, expected_error).unwrap();
}
