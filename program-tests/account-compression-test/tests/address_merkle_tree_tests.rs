#![cfg(feature = "test-sbf")]

use std::mem;

use account_compression::{
    errors::AccountCompressionErrorCode,
    state::QueueAccount,
    utils::constants::{ADDRESS_MERKLE_TREE_CANOPY_DEPTH, ADDRESS_MERKLE_TREE_HEIGHT},
    AddressMerkleTreeAccount, AddressMerkleTreeConfig, AddressQueueConfig, ID, SAFETY_MARGIN,
};
use anchor_lang::error::ErrorCode;
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField, UniformRand};
use light_account_checks::error::AccountError;
use light_bounded_vec::BoundedVecError;
use light_client::indexer::AddressMerkleTreeAccounts;
use light_concurrent_merkle_tree::errors::ConcurrentMerkleTreeError;
use light_hash_set::{HashSet, HashSetError};
use light_hasher::{bigint::bigint_to_be_bytes_array, Poseidon};
use light_indexed_merkle_tree::errors::IndexedMerkleTreeError;
use light_merkle_tree_metadata::errors::MerkleTreeMetadataError;
use light_program_test::{
    accounts::address_tree::create_initialize_address_merkle_tree_and_queue_instruction,
    indexer::address_tree::AddressMerkleTreeBundle, program_test::LightProgramTest,
    utils::assert::assert_rpc_error, ProgramTestConfig,
};
use light_test_utils::{
    address::insert_addresses,
    address_tree_rollover::{
        assert_rolled_over_address_merkle_tree_and_queue, perform_address_merkle_tree_roll_over,
        set_address_merkle_tree_next_index,
    },
    airdrop_lamports, create_account_instruction,
    create_address_merkle_tree_and_queue_account_with_assert, get_hash_set,
    get_indexed_merkle_tree,
    test_forester::{empty_address_queue_test, update_merkle_tree},
    Rpc, RpcError,
};
use num_bigint::ToBigUint;
use rand::thread_rng;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};

/// Tests insertion of addresses to the queue, dequeuing and Merkle tree update.
/// 1. create address Merkle tree and queue accounts
/// 2. inserts two addresses to the queue
/// 3. inserts two addresses into the address Merkle tree
/// 4. insert third address
async fn address_queue_and_tree_functional(
    merkle_tree_config: &AddressMerkleTreeConfig,
    queue_config: &AddressQueueConfig,
) {
    // CHECK: 1 create address Merkle tree and queue accounts
    let (mut context, _, mut address_merkle_tree_bundle) =
        test_setup_with_address_merkle_tree(merkle_tree_config, queue_config).await;
    let payer = context.get_payer().insecure_clone();
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
        get_hash_set::<QueueAccount, LightProgramTest>(&mut context, address_queue_pubkey).await
    };

    assert!(address_queue.contains(&address1, None).unwrap());
    assert!(address_queue.contains(&address2, None).unwrap());

    // CHECK: 3 inserts two addresses into the address Merkle tree
    empty_address_queue_test(
        &payer,
        &mut context,
        &mut address_merkle_tree_bundle,
        true,
        0,
        false,
    )
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
        get_hash_set::<QueueAccount, LightProgramTest>(&mut context, address_queue_pubkey).await
    };
    address_queue
        .find_element(&address3, None)
        .unwrap()
        .unwrap();
    // CHECK: 4 insert third address which is inbetween the first two addresses
    empty_address_queue_test(
        &payer,
        &mut context,
        &mut address_merkle_tree_bundle,
        true,
        0,
        false,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_address_queue_and_tree_functional_default() {
    address_queue_and_tree_functional(
        &AddressMerkleTreeConfig::default(),
        &AddressQueueConfig::default(),
    )
    .await
}

#[tokio::test]
async fn test_address_queue_and_tree_functional_custom() {
    for changelog_size in [1, 1000] {
        for roots_size in [1000] {
            if roots_size < changelog_size {
                continue;
            }
            for queue_capacity in [7901] {
                for address_changelog_size in (750..1000).step_by(250) {
                    address_queue_and_tree_functional(
                        &AddressMerkleTreeConfig {
                            height: ADDRESS_MERKLE_TREE_HEIGHT as u32,
                            changelog_size,
                            roots_size,
                            canopy_depth: ADDRESS_MERKLE_TREE_CANOPY_DEPTH,
                            address_changelog_size,
                            network_fee: Some(5000),
                            rollover_threshold: Some(95),
                            close_threshold: None,
                        },
                        &AddressQueueConfig {
                            capacity: queue_capacity,
                            sequence_threshold: roots_size + SAFETY_MARGIN,
                            network_fee: None,
                        },
                    )
                    .await;
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn initialize_address_merkle_tree_and_queue<R: Rpc>(
    context: &mut R,
    payer: &Keypair,
    merkle_tree_keypair: &Keypair,
    queue_keypair: &Keypair,
    merkle_tree_config: &AddressMerkleTreeConfig,
    queue_config: &AddressQueueConfig,
    merkle_tree_size: usize,
    queue_size: usize,
) -> Result<Signature, RpcError> {
    let queue_account_create_ix = create_account_instruction(
        &payer.pubkey(),
        queue_size,
        context
            .get_minimum_balance_for_rent_exemption(queue_size)
            .await
            .unwrap(),
        &ID,
        Some(queue_keypair),
    );
    let mt_account_create_ix = create_account_instruction(
        &payer.pubkey(),
        merkle_tree_size,
        context
            .get_minimum_balance_for_rent_exemption(merkle_tree_size)
            .await
            .unwrap(),
        &ID,
        Some(merkle_tree_keypair),
    );

    let instruction = create_initialize_address_merkle_tree_and_queue_instruction(
        0,
        payer.pubkey(),
        None,
        None,
        Some(Pubkey::new_unique()),
        merkle_tree_keypair.pubkey(),
        queue_keypair.pubkey(),
        merkle_tree_config.clone(),
        queue_config.clone(),
    );
    let transaction = Transaction::new_signed_with_payer(
        &[queue_account_create_ix, mt_account_create_ix, instruction],
        Some(&payer.pubkey()),
        &vec![&payer, &queue_keypair, &merkle_tree_keypair],
        context.get_latest_blockhash().await.unwrap().0,
    );

    context.process_transaction(transaction.clone()).await
}

#[tokio::test]
async fn test_address_queue_and_tree_invalid_sizes() {
    let config = ProgramTestConfig {
        skip_protocol_init: true,
        with_prover: false,
        ..Default::default()
    };
    let mut context = LightProgramTest::new(config).await.unwrap();

    let payer = context.get_payer().insecure_clone();

    let address_merkle_tree_keypair = Keypair::new();
    let address_queue_keypair = Keypair::new();

    let queue_config = AddressQueueConfig::default();
    let merkle_tree_config = AddressMerkleTreeConfig::default();

    let valid_queue_size =
        QueueAccount::size(account_compression::utils::constants::ADDRESS_QUEUE_VALUES as usize)
            .unwrap();
    let valid_tree_size = AddressMerkleTreeAccount::size(
        merkle_tree_config.height as usize,
        merkle_tree_config.changelog_size as usize,
        merkle_tree_config.roots_size as usize,
        merkle_tree_config.canopy_depth as usize,
        merkle_tree_config.address_changelog_size as usize,
    );

    // NOTE: Starting from 0 to the account struct size triggers a panic in Anchor
    // macros (sadly, not assertable...), which happens earlier than our
    // serialization error.
    // Our recoverable error is thrown for ranges from the struct size
    // (+ discriminator) up to the expected account size.

    // Invalid MT size + invalid queue size.
    for tree_size in
        (8 + mem::size_of::<AddressMerkleTreeAccount>()..=valid_tree_size).step_by(200_000)
    {
        for queue_size in (8 + mem::size_of::<QueueAccount>()..=valid_queue_size).step_by(50_000) {
            let result = initialize_address_merkle_tree_and_queue(
                &mut context,
                &payer,
                &address_merkle_tree_keypair,
                &address_queue_keypair,
                &merkle_tree_config,
                &queue_config,
                tree_size,
                queue_size,
            )
            .await;
            assert_rpc_error(result, 2, AccountError::InvalidAccountSize.into()).unwrap()
        }
    }
    // Invalid MT size + valid queue size.
    for tree_size in
        (8 + mem::size_of::<AddressMerkleTreeAccount>()..=valid_tree_size).step_by(200_000)
    {
        let result = initialize_address_merkle_tree_and_queue(
            &mut context,
            &payer,
            &address_merkle_tree_keypair,
            &address_queue_keypair,
            &merkle_tree_config,
            &queue_config,
            tree_size,
            valid_queue_size,
        )
        .await;
        assert_rpc_error(result, 2, AccountError::InvalidAccountSize.into()).unwrap()
    }
    // Valid MT size + invalid queue size.
    for queue_size in (8 + mem::size_of::<QueueAccount>()..=valid_queue_size).step_by(50_000) {
        let result = initialize_address_merkle_tree_and_queue(
            &mut context,
            &payer,
            &address_merkle_tree_keypair,
            &address_queue_keypair,
            &merkle_tree_config,
            &queue_config,
            valid_tree_size,
            queue_size,
        )
        .await;
        assert_rpc_error(result, 2, AccountError::InvalidAccountSize.into()).unwrap()
    }
}

/// Tries to initzalize Merkle tree and queue with unsupported configuration
/// parameters:
///
/// 1. Merkle tree height (different than 26).
/// 2. Merkle tree canopy depth (different than 10).
/// 3. Merkle tree changelog size (zero).
/// 4. Merkle tree roots size (zero).
/// 5. Merkle tree close threshold (any).
/// 6. Queue sequence threshold (lower than roots + safety margin).
#[tokio::test]
async fn test_address_queue_and_tree_invalid_config() {
    let config = ProgramTestConfig {
        skip_protocol_init: true,
        with_prover: false,
        ..Default::default()
    };
    let mut context = LightProgramTest::new(config).await.unwrap();

    let payer = context.get_payer().insecure_clone();

    let address_merkle_tree_keypair = Keypair::new();
    let address_queue_keypair = Keypair::new();

    let queue_config = AddressQueueConfig::default();
    let merkle_tree_config = AddressMerkleTreeConfig::default();

    let queue_size =
        QueueAccount::size(account_compression::utils::constants::ADDRESS_QUEUE_VALUES as usize)
            .unwrap();
    let tree_size = AddressMerkleTreeAccount::size(
        merkle_tree_config.height as usize,
        merkle_tree_config.changelog_size as usize,
        merkle_tree_config.roots_size as usize,
        merkle_tree_config.canopy_depth as usize,
        merkle_tree_config.address_changelog_size as usize,
    );

    for invalid_height in (0..26).step_by(5) {
        let mut merkle_tree_config = merkle_tree_config.clone();
        merkle_tree_config.height = invalid_height;
        let result = initialize_address_merkle_tree_and_queue(
            &mut context,
            &payer,
            &address_merkle_tree_keypair,
            &address_queue_keypair,
            &merkle_tree_config,
            &queue_config,
            tree_size,
            queue_size,
        )
        .await;
        println!("Invalid result: {}", result.as_ref().unwrap_err());
        assert_rpc_error(
            result,
            2,
            AccountCompressionErrorCode::UnsupportedHeight.into(),
        )
        .unwrap();
    }
    for invalid_height in (27..50).step_by(5) {
        let mut merkle_tree_config = merkle_tree_config.clone();
        merkle_tree_config.height = invalid_height;
        let result = initialize_address_merkle_tree_and_queue(
            &mut context,
            &payer,
            &address_merkle_tree_keypair,
            &address_queue_keypair,
            &merkle_tree_config,
            &queue_config,
            tree_size,
            queue_size,
        )
        .await;
        assert_rpc_error(
            result,
            2,
            AccountCompressionErrorCode::UnsupportedHeight.into(),
        )
        .unwrap();
    }
    for invalid_canopy_depth in (0..10).step_by(3) {
        let mut merkle_tree_config = merkle_tree_config.clone();
        merkle_tree_config.canopy_depth = invalid_canopy_depth;
        let result = initialize_address_merkle_tree_and_queue(
            &mut context,
            &payer,
            &address_merkle_tree_keypair,
            &address_queue_keypair,
            &merkle_tree_config,
            &queue_config,
            tree_size,
            queue_size,
        )
        .await;
        assert_rpc_error(
            result,
            2,
            AccountCompressionErrorCode::UnsupportedCanopyDepth.into(),
        )
        .unwrap();
    }
    {
        let mut merkle_tree_config = merkle_tree_config.clone();
        merkle_tree_config.changelog_size = 0;
        let tree_size = AddressMerkleTreeAccount::size(
            merkle_tree_config.height as usize,
            merkle_tree_config.changelog_size as usize,
            merkle_tree_config.roots_size as usize,
            merkle_tree_config.canopy_depth as usize,
            merkle_tree_config.address_changelog_size as usize,
        );
        let result = initialize_address_merkle_tree_and_queue(
            &mut context,
            &payer,
            &address_merkle_tree_keypair,
            &address_queue_keypair,
            &merkle_tree_config,
            &queue_config,
            tree_size,
            queue_size,
        )
        .await;
        assert_rpc_error(result, 2, ConcurrentMerkleTreeError::ChangelogZero.into()).unwrap();
    }
    {
        let mut merkle_tree_config = merkle_tree_config.clone();
        merkle_tree_config.roots_size = 0;
        let tree_size = AddressMerkleTreeAccount::size(
            merkle_tree_config.height as usize,
            merkle_tree_config.changelog_size as usize,
            merkle_tree_config.roots_size as usize,
            merkle_tree_config.canopy_depth as usize,
            merkle_tree_config.address_changelog_size as usize,
        );
        let result = initialize_address_merkle_tree_and_queue(
            &mut context,
            &payer,
            &address_merkle_tree_keypair,
            &address_queue_keypair,
            &merkle_tree_config,
            &queue_config,
            tree_size,
            queue_size,
        )
        .await;
        assert_rpc_error(result, 2, ConcurrentMerkleTreeError::RootsZero.into()).unwrap();
    }
    for invalid_close_threshold in (0..100).step_by(20) {
        let mut merkle_tree_config = merkle_tree_config.clone();
        merkle_tree_config.close_threshold = Some(invalid_close_threshold);
        let result = initialize_address_merkle_tree_and_queue(
            &mut context,
            &payer,
            &address_merkle_tree_keypair,
            &address_queue_keypair,
            &merkle_tree_config,
            &queue_config,
            tree_size,
            queue_size,
        )
        .await;
        assert_rpc_error(
            result,
            2,
            AccountCompressionErrorCode::UnsupportedCloseThreshold.into(),
        )
        .unwrap();
    }
    for invalid_sequence_threshold in
        (0..merkle_tree_config.roots_size + SAFETY_MARGIN).step_by(200)
    {
        let mut queue_config = queue_config.clone();
        queue_config.sequence_threshold = invalid_sequence_threshold;
        let result = initialize_address_merkle_tree_and_queue(
            &mut context,
            &payer,
            &address_merkle_tree_keypair,
            &address_queue_keypair,
            &merkle_tree_config,
            &queue_config,
            tree_size,
            queue_size,
        )
        .await;
        assert_rpc_error(
            result,
            2,
            AccountCompressionErrorCode::InvalidSequenceThreshold.into(),
        )
        .unwrap();
    }
}

/// Try to insert an address to the tree while providing invalid parameters.
///
/// Such invalid insertion needs to be performed manually, without relayer's
/// help (which would always insert that nullifier correctly).
/// Tests:
/// 1. cannot insert the same address twice
/// 2. cannot insert an address with an invalid low address
///    2.1 cannot insert an address with an invalid low address (NewElementGreaterOrEqualToNextElement)
///    2.2 cannot insert an address with an invalid low address (LowElementGreaterOrEqualToNewElement)
///    3.1 invalid value index (element does not exist)
///    3.2 invalid value index (element has a sequence number)
/// 4. invalid low element index
/// 5. invalid low element value
/// 6. invalid low element next index
/// 7. invalid low element next value
/// 8. invalid low element proof
/// 9. invalid changelog index (lower)
/// 10. invalid changelog index (higher)
/// 11. invalid indexed changelog index (higher)
/// 12. invalid queue account
/// 13. invalid Merkle tree account
/// 14. non-associated Merkle tree
async fn update_address_merkle_tree_failing_tests(
    merkle_tree_config: &AddressMerkleTreeConfig,
    queue_config: &AddressQueueConfig,
) {
    let (mut context, payer, mut address_merkle_tree_bundle) =
        test_setup_with_address_merkle_tree(merkle_tree_config, queue_config).await;
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
        addresses.clone(),
    )
    .await
    .unwrap();
    empty_address_queue_test(
        &payer,
        &mut context,
        &mut address_merkle_tree_bundle,
        true,
        0,
        false,
    )
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
        get_hash_set::<QueueAccount, LightProgramTest>(&mut context, address_queue_pubkey).await
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
        .find_low_element_for_nonexistent(&address.value_biguint())
        .unwrap();
    // Get the Merkle proof for updating low element.
    let low_element_proof = address_merkle_tree_bundle
        .get_proof_of_leaf(low_element.index, false)
        .unwrap();
    let value_index = address_hashset_index;

    // CHECK: 3.1 invalid value index (value doesn't exist)
    let invalid_value_index = 10;
    // unwraps on a None value onchain.
    update_merkle_tree(
        &mut context,
        &payer,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        invalid_value_index,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.clone().try_into().unwrap(),
        None,
        None,
        true,
        0,
        false,
    )
    .await
    .unwrap_err();
    // CHECK: 3.2 invalid value index (value has a sequence number)
    let invalid_value_index = 0;
    // unwraps on a None value onchain.
    update_merkle_tree(
        &mut context,
        &payer,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        invalid_value_index,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.clone().try_into().unwrap(),
        None,
        None,
        true,
        0,
        false,
    )
    .await
    .unwrap_err();
    // CHECK: 4 invalid low element index
    let invalid_lower_element_index = low_element.index - 1;
    let error_invalid_low_element_index = update_merkle_tree(
        &mut context,
        &payer,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        value_index,
        invalid_lower_element_index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.clone().try_into().unwrap(),
        None,
        None,
        true,
        0,
        false,
    )
    .await;
    assert_rpc_error(
        error_invalid_low_element_index,
        0,
        ConcurrentMerkleTreeError::InvalidProof([0; 32], [0; 32]).into(),
    )
    .unwrap();

    // CHECK: 5 invalid low element value
    let invalid_low_element_value = [0u8; 32];
    let error_invalid_low_element_value = update_merkle_tree(
        &mut context,
        &payer,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        value_index,
        low_element.index as u64,
        invalid_low_element_value,
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.clone().try_into().unwrap(),
        None,
        None,
        true,
        0,
        false,
    )
    .await;
    assert_rpc_error(
        error_invalid_low_element_value,
        0,
        ConcurrentMerkleTreeError::InvalidProof([0; 32], [0; 32]).into(),
    )
    .unwrap();

    // CHECK: 6 invalid low element next index
    let invalid_low_element_next_index = 1;
    let error_invalid_low_element_next_index = update_merkle_tree(
        &mut context,
        &payer,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        value_index,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        invalid_low_element_next_index,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.clone().try_into().unwrap(),
        None,
        None,
        true,
        0,
        false,
    )
    .await;
    assert_rpc_error(
        error_invalid_low_element_next_index,
        0,
        ConcurrentMerkleTreeError::InvalidProof([0; 32], [0; 32]).into(),
    )
    .unwrap();

    // CHECK: 7 invalid low element next value
    let invalid_low_element_next_value = [9u8; 32];
    let error_invalid_low_element_next_value = update_merkle_tree(
        &mut context,
        &payer,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        value_index,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        invalid_low_element_next_value,
        low_element_proof.clone().try_into().unwrap(),
        None,
        None,
        true,
        0,
        false,
    )
    .await;
    assert_rpc_error(
        error_invalid_low_element_next_value,
        0,
        ConcurrentMerkleTreeError::InvalidProof([0; 32], [0; 32]).into(),
    )
    .unwrap();

    // CHECK: 8 invalid low element proof
    let mut invalid_low_element_proof = low_element_proof.clone();
    invalid_low_element_proof.get_mut(0).unwrap()[0] = 0;
    let error_invalid_low_element_proof = update_merkle_tree(
        &mut context,
        &payer,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        value_index,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        invalid_low_element_proof.try_into().unwrap(),
        None,
        None,
        true,
        0,
        false,
    )
    .await;
    assert_rpc_error(
        error_invalid_low_element_proof,
        0,
        ConcurrentMerkleTreeError::InvalidProof([0; 32], [0; 32]).into(),
    )
    .unwrap();
    let address_merkle_tree = get_indexed_merkle_tree::<
        AddressMerkleTreeAccount,
        LightProgramTest,
        Poseidon,
        usize,
        26,
        16,
    >(&mut context, address_merkle_tree_pubkey)
    .await;

    let changelog_index = address_merkle_tree.changelog_index();

    if merkle_tree_config.changelog_size >= 2 {
        // CHECK: 9 invalid changelog index (lower)
        let invalid_changelog_index_low = changelog_index - 2;
        let error_invalid_changelog_index_low = update_merkle_tree(
            &mut context,
            &payer,
            address_queue_pubkey,
            address_merkle_tree_pubkey,
            value_index,
            low_element.index as u64,
            bigint_to_be_bytes_array(&low_element.value).unwrap(),
            low_element.next_index as u64,
            bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
            low_element_proof.clone().try_into().unwrap(),
            Some(invalid_changelog_index_low as u16),
            None,
            true,
            0,
            false,
        )
        .await;
        assert_rpc_error(
            error_invalid_changelog_index_low,
            0,
            ConcurrentMerkleTreeError::CannotUpdateLeaf.into(),
        )
        .unwrap();

        // CHECK: 10 invalid changelog index (higher)
        let invalid_changelog_index_high = changelog_index + 2;
        let error_invalid_changelog_index_high = update_merkle_tree(
            &mut context,
            &payer,
            address_queue_pubkey,
            address_merkle_tree_pubkey,
            value_index,
            low_element.index as u64,
            bigint_to_be_bytes_array(&low_element.value).unwrap(),
            low_element.next_index as u64,
            bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
            low_element_proof.clone().try_into().unwrap(),
            Some(invalid_changelog_index_high as u16),
            None,
            true,
            0,
            false,
        )
        .await;
        assert_rpc_error(
            error_invalid_changelog_index_high,
            0,
            BoundedVecError::IterFromOutOfBounds.into(),
        )
        .unwrap();
    }

    let indexed_changelog_index = address_merkle_tree.indexed_changelog_index();

    if merkle_tree_config.address_changelog_size >= 2 {
        // CHECK: 11 invalid indexed changelog index (higher)
        let invalid_indexed_changelog_index_high = indexed_changelog_index + 1;
        let error_invalid_indexed_changelog_index_high = update_merkle_tree(
            &mut context,
            &payer,
            address_queue_pubkey,
            address_merkle_tree_pubkey,
            value_index,
            low_element.index as u64,
            bigint_to_be_bytes_array(&low_element.value).unwrap(),
            low_element.next_index as u64,
            bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
            low_element_proof.clone().try_into().unwrap(),
            None,
            Some(invalid_indexed_changelog_index_high as u16),
            true,
            0,
            false,
        )
        .await;
        assert_rpc_error(
            error_invalid_indexed_changelog_index_high,
            0,
            BoundedVecError::IterFromOutOfBounds.into(),
        )
        .unwrap();
    }

    // CHECK: 12 invalid queue account
    let invalid_queue = address_merkle_tree_pubkey;
    let error_invalid_queue = update_merkle_tree(
        &mut context,
        &payer,
        invalid_queue,
        address_merkle_tree_pubkey,
        value_index,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.clone().try_into().unwrap(),
        None,
        None,
        true,
        0,
        false,
    )
    .await;
    assert_rpc_error(
        error_invalid_queue,
        0,
        ErrorCode::AccountDiscriminatorMismatch.into(),
    )
    .unwrap();

    // CHECK: 13 invalid Merkle tree account
    let indexed_changelog_index = address_merkle_tree.indexed_changelog_index();
    let invalid_merkle_tree = address_queue_pubkey;
    let error_invalid_merkle_tree = update_merkle_tree(
        &mut context,
        &payer,
        address_queue_pubkey,
        invalid_merkle_tree,
        value_index,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.clone().try_into().unwrap(),
        Some(changelog_index as u16),
        Some(indexed_changelog_index as u16),
        true,
        0,
        false,
    )
    .await;
    assert_rpc_error(
        error_invalid_merkle_tree,
        0,
        ErrorCode::AccountDiscriminatorMismatch.into(),
    )
    .unwrap();

    let invalid_address_merkle_tree_keypair = Keypair::new();
    let invalid_address_queue_keypair = Keypair::new();
    create_address_merkle_tree_and_queue_account_with_assert(
        &payer,
        false,
        &mut context,
        &invalid_address_merkle_tree_keypair,
        &invalid_address_queue_keypair,
        None,
        None,
        merkle_tree_config,
        queue_config,
        2,
    )
    .await
    .unwrap();

    // CHECK: 14 non-associated Merkle tree
    let invalid_merkle_tree = invalid_address_merkle_tree_keypair.pubkey();
    let error_non_associated_merkle_tree = update_merkle_tree(
        &mut context,
        &payer,
        address_queue_pubkey,
        invalid_merkle_tree,
        value_index,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.clone().try_into().unwrap(),
        Some(changelog_index as u16),
        None,
        true,
        0,
        false,
    )
    .await;
    assert_rpc_error(
        error_non_associated_merkle_tree,
        0,
        AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated.into(),
    )
    .unwrap();
}

#[tokio::test]
async fn update_address_merkle_tree_failing_tests_default() {
    update_address_merkle_tree_failing_tests(
        &AddressMerkleTreeConfig::default(),
        &AddressQueueConfig::default(),
    )
    .await
}

async fn update_address_merkle_tree_wrap_around(
    merkle_tree_config: &AddressMerkleTreeConfig,
    queue_config: &AddressQueueConfig,
) {
    let (mut context, payer, mut address_merkle_tree_bundle) =
        test_setup_with_address_merkle_tree(merkle_tree_config, queue_config).await;
    let address_queue_pubkey = address_merkle_tree_bundle.accounts.queue;
    let address_merkle_tree_pubkey = address_merkle_tree_bundle.accounts.merkle_tree;
    // Insert a pair of addresses, correctly. Just do it with relayer.
    let address1 = 30_u32.to_biguint().unwrap();
    let address2 = 10_u32.to_biguint().unwrap();
    let addresses: Vec<[u8; 32]> = vec![
        bigint_to_be_bytes_array(&address1).unwrap(),
        bigint_to_be_bytes_array(&address2).unwrap(),
    ];

    let (low_element, low_element_next_value) = address_merkle_tree_bundle
        .find_low_element_for_nonexistent(&address1)
        .unwrap();
    // Get the Merkle proof for updating low element.
    let low_element_proof = address_merkle_tree_bundle
        .get_proof_of_leaf(low_element.index, false)
        .unwrap();

    // Wrap around the indexed changelog with conflicting elements.
    let mut rng = thread_rng();
    for _ in (0..merkle_tree_config.address_changelog_size).step_by(10) {
        let addresses: Vec<[u8; 32]> = (0..10)
            .map(|_| {
                Fr::rand(&mut rng)
                    .into_bigint()
                    .to_bytes_be()
                    .try_into()
                    .unwrap()
            })
            .collect::<Vec<_>>();
        insert_addresses(
            &mut context,
            address_queue_pubkey,
            address_merkle_tree_pubkey,
            addresses,
        )
        .await
        .unwrap();
        empty_address_queue_test(
            &payer,
            &mut context,
            &mut address_merkle_tree_bundle,
            true,
            0,
            false,
        )
        .await
        .unwrap();
    }

    insert_addresses(
        &mut context,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        addresses.clone(),
    )
    .await
    .unwrap();

    let address_queue = unsafe {
        get_hash_set::<QueueAccount, LightProgramTest>(&mut context, address_queue_pubkey).await
    };
    let value_index = address_queue
        .find_element_index(&address1, None)
        .unwrap()
        .unwrap();

    let error = update_merkle_tree(
        &mut context,
        &payer,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        value_index as u16,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.clone().try_into().unwrap(),
        None,
        None,
        true,
        0,
        false,
    )
    .await;
    assert_rpc_error(
        error,
        0,
        ConcurrentMerkleTreeError::InvalidProof([0; 32], [0; 32]).into(),
    )
    .unwrap();
}

#[tokio::test]
async fn update_address_merkle_tree_wrap_around_default() {
    update_address_merkle_tree_wrap_around(
        &AddressMerkleTreeConfig::default(),
        &AddressQueueConfig::default(),
    )
    .await
}

#[tokio::test]
async fn update_address_merkle_tree_wrap_around_custom() {
    let changelog_size = 250;
    let queue_capacity = 5003;
    let roots_size = changelog_size * 2;

    for address_changelog_size in (250..1000).step_by(250) {
        println!(
            "changelog_size {} queue_capacity {} address_changelog_size {}",
            changelog_size, queue_capacity, address_changelog_size
        );
        update_address_merkle_tree_wrap_around(
            &AddressMerkleTreeConfig {
                height: ADDRESS_MERKLE_TREE_HEIGHT as u32,
                changelog_size,
                roots_size,
                canopy_depth: ADDRESS_MERKLE_TREE_CANOPY_DEPTH,
                address_changelog_size,
                network_fee: Some(5000),
                rollover_threshold: Some(95),
                close_threshold: None,
            },
            &AddressQueueConfig {
                capacity: queue_capacity,
                sequence_threshold: roots_size + SAFETY_MARGIN,
                network_fee: None,
            },
        )
        .await;
    }
}

/// Tests address Merkle tree and queue rollover.
/// 1. Not ready for rollover after init.
/// 2. Not ready for rollover after setting next index to required value - 1.
/// 3. Merkle tree and queue not associated (Invalid queue).
/// 4. Merkle tree and queue not associated (Invalid Merkle tree).
/// 5. Successful rollover.
/// 6. Attempt to rollover already rolled over Queue and Merkle tree.
async fn address_merkle_tree_and_queue_rollover(
    merkle_tree_config: &AddressMerkleTreeConfig,
    queue_config: &AddressQueueConfig,
) {
    let (mut context, payer, bundle) =
        test_setup_with_address_merkle_tree(merkle_tree_config, queue_config).await;
    let address_merkle_tree_pubkey = bundle.accounts.merkle_tree;
    let address_queue_pubkey = bundle.accounts.queue;
    let address_merkle_tree_keypair_2 = Keypair::new();
    let address_queue_keypair_2 = Keypair::new();
    create_address_merkle_tree_and_queue_account_with_assert(
        &payer,
        false,
        &mut context,
        &address_merkle_tree_keypair_2,
        &address_queue_keypair_2,
        None,
        None,
        merkle_tree_config,
        queue_config,
        2,
    )
    .await
    .unwrap();
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
        merkle_tree_config,
        queue_config,
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
        merkle_tree_config,
        queue_config,
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
        merkle_tree_config,
        queue_config,
    )
    .await;

    assert_rpc_error(
        result,
        2,
        MerkleTreeMetadataError::MerkleTreeAndQueueNotAssociated.into(),
    )
    .unwrap();

    // CHECK 4: Merkle tree and queue not associated invalid Merkle tree.
    let result = perform_address_merkle_tree_roll_over(
        &mut context,
        &new_queue_keypair,
        &new_address_merkle_tree_keypair,
        &address_merkle_tree_keypair_2.pubkey(),
        &address_queue_pubkey,
        merkle_tree_config,
        queue_config,
    )
    .await;

    assert_rpc_error(
        result,
        2,
        MerkleTreeMetadataError::MerkleTreeAndQueueNotAssociated.into(),
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
        merkle_tree_config,
        queue_config,
    )
    .await
    .unwrap();
    let payer: Keypair = context.get_payer().insecure_clone();
    assert_rolled_over_address_merkle_tree_and_queue(
        &payer.pubkey(),
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
        merkle_tree_config,
        queue_config,
    )
    .await;

    assert_rpc_error(
        result,
        2,
        MerkleTreeMetadataError::MerkleTreeAlreadyRolledOver.into(),
    )
    .unwrap();
}

#[tokio::test]
async fn test_address_merkle_tree_and_queue_rollover_default() {
    address_merkle_tree_and_queue_rollover(
        &AddressMerkleTreeConfig::default(),
        &AddressQueueConfig::default(),
    )
    .await
}

#[tokio::test]
async fn test_address_merkle_tree_and_queue_rollover_custom() {
    for changelog_size in [1, 1000] {
        for roots_size in [1, 1000] {
            if roots_size < changelog_size {
                continue;
            }
            for queue_capacity in [5003] {
                for address_changelog_size in (250..500).step_by(250) {
                    address_merkle_tree_and_queue_rollover(
                        &AddressMerkleTreeConfig {
                            height: ADDRESS_MERKLE_TREE_HEIGHT as u32,
                            changelog_size,
                            roots_size,
                            canopy_depth: ADDRESS_MERKLE_TREE_CANOPY_DEPTH,
                            address_changelog_size,
                            network_fee: Some(5000),
                            rollover_threshold: Some(95),
                            close_threshold: None,
                        },
                        &AddressQueueConfig {
                            capacity: queue_capacity,
                            sequence_threshold: roots_size + SAFETY_MARGIN,
                            network_fee: None,
                        },
                    )
                    .await;
                }
            }
        }
    }
}

pub async fn test_setup_with_address_merkle_tree(
    merkle_tree_config: &AddressMerkleTreeConfig,
    queue_config: &AddressQueueConfig,
) -> (
    LightProgramTest, // rpc
    Keypair,          // payer
    AddressMerkleTreeBundle,
) {
    let config = ProgramTestConfig {
        skip_protocol_init: true,
        with_prover: false,
        ..Default::default()
    };
    let mut context = LightProgramTest::new(config).await.unwrap();

    let payer = context.get_payer().insecure_clone();

    let address_merkle_tree_keypair = Keypair::new();
    let address_queue_keypair = Keypair::new();
    create_address_merkle_tree_and_queue_account_with_assert(
        &payer,
        false,
        &mut context,
        &address_merkle_tree_keypair,
        &address_queue_keypair,
        None,
        None,
        merkle_tree_config,
        queue_config,
        1,
    )
    .await
    .unwrap();

    let address_merkle_tree_bundle = AddressMerkleTreeBundle::new_v1(AddressMerkleTreeAccounts {
        merkle_tree: address_merkle_tree_keypair.pubkey(),
        queue: address_queue_keypair.pubkey(),
    })
    .unwrap();
    (context, payer, address_merkle_tree_bundle)
}

pub async fn test_with_invalid_low_element(
    context: &mut LightProgramTest,
    address_queue_pubkey: Pubkey,
    address_merkle_tree_pubkey: Pubkey,
    address_queue: &HashSet,
    address_merkle_tree_bundle: &AddressMerkleTreeBundle,
    index: usize,
    expected_error: u32,
) {
    let payer = context.get_payer().insecure_clone();
    let (_, address_hashset_index) = address_queue.first_no_seq().unwrap().unwrap();
    let low_element = address_merkle_tree_bundle
        .indexed_array_v1()
        .unwrap()
        .get(index)
        .unwrap();
    let low_element_next_value = address_merkle_tree_bundle
        .indexed_array_v1()
        .unwrap()
        .get(low_element.next_index())
        .unwrap()
        .value
        .clone();
    // Get the Merkle proof for updating low element.
    let low_element_proof = address_merkle_tree_bundle
        .get_proof_of_leaf(low_element.index, false)
        .unwrap();
    let value_index = address_hashset_index;

    // unwraps on a None value onchain.
    let error_invalid_low_element = update_merkle_tree(
        context,
        &payer,
        address_queue_pubkey,
        address_merkle_tree_pubkey,
        value_index,
        low_element.index as u64,
        bigint_to_be_bytes_array(&low_element.value).unwrap(),
        low_element.next_index as u64,
        bigint_to_be_bytes_array(&low_element_next_value).unwrap(),
        low_element_proof.clone().try_into().unwrap(),
        None,
        None,
        true,
        0,
        false,
    )
    .await;
    assert_rpc_error(error_invalid_low_element, 0, expected_error).unwrap();
}
