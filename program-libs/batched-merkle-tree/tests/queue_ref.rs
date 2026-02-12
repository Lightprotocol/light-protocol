mod test_helpers;

use light_batched_merkle_tree::{queue::BatchedQueueAccount, queue_ref::BatchedQueueRef};
use light_compressed_account::{pubkey::Pubkey, QueueType, OUTPUT_STATE_QUEUE_TYPE_V2};
use light_merkle_tree_metadata::{errors::MerkleTreeMetadataError, queue::QueueMetadata};
use test_helpers::{account_builders::QueueAccountBuilder, assertions::*};

#[test]
fn test_queue_ref_matches_mutable() {
    let mut account_data = vec![0u8; 1000];
    let associated_merkle_tree = Pubkey::new_unique();
    let queue_metadata = QueueMetadata {
        associated_merkle_tree,
        queue_type: QueueType::OutputStateV2 as u64,
        ..Default::default()
    };
    let batch_size = 4;
    let zkp_batch_size = 2;
    let bloom_filter_capacity = 0;
    let num_iters = 0;
    let pubkey = Pubkey::new_unique();
    let tree_capacity = 16;

    // Initialize via mutable path.
    let _account = BatchedQueueAccount::init(
        &mut account_data,
        queue_metadata,
        batch_size,
        zkp_batch_size,
        num_iters,
        bloom_filter_capacity,
        pubkey,
        tree_capacity,
    )
    .unwrap();

    // Collect expected values from the mutable path.
    let expected_metadata;
    let expected_assoc_tree;
    let expected_pubkey;
    {
        let queue_mut = BatchedQueueAccount::output_from_bytes(&mut account_data).unwrap();
        expected_metadata = *queue_mut.get_metadata();
        expected_assoc_tree = queue_mut.get_metadata().metadata.associated_merkle_tree;
        expected_pubkey = *queue_mut.pubkey();
    }

    // Read via immutable ref.
    let queue_ref = BatchedQueueRef::output_from_bytes(&account_data).unwrap();

    // Metadata should match (use Deref trait).
    assert_eq!(*queue_ref, expected_metadata);
    assert_eq!(
        queue_ref.metadata.associated_merkle_tree,
        expected_assoc_tree
    );
    assert_eq!(*queue_ref.pubkey(), expected_pubkey);

    // Association check should work.
    queue_ref
        .check_is_associated(&associated_merkle_tree)
        .unwrap();

    // Test check_is_associated with wrong pubkey returns error.
    assert!(queue_ref
        .check_is_associated(&Pubkey::new_unique())
        .is_err());
}

// ============================================================================
// New comprehensive tests for 100% coverage
// ============================================================================

#[test]
fn test_queue_ref_deserialization_success() {
    let associated_tree = Pubkey::new_unique();
    let (data, _pubkey) = QueueAccountBuilder::output_queue()
        .with_associated_tree(associated_tree)
        .with_batch_size(10)
        .build();

    let queue_ref = BatchedQueueRef::output_from_bytes(&data)
        .expect("Should deserialize valid output queue");

    // Verify metadata fields
    assert_eq!(
        queue_ref.metadata.associated_merkle_tree,
        associated_tree,
        "Associated tree should match"
    );
    assert_eq!(
        queue_ref.batch_metadata.batch_size,
        10,
        "Batch size should match"
    );
}

#[test]
fn test_queue_ref_deserialization_errors() {
    // Test 1: Bad discriminator
    let (data, _pubkey) = QueueAccountBuilder::output_queue()
        .build_with_bad_discriminator();
    let result = BatchedQueueRef::output_from_bytes(&data);
    assert_account_error(result, "Bad discriminator should fail");

    // Test 2: Insufficient size
    let (data, _pubkey) = QueueAccountBuilder::output_queue().build_too_small();
    let result = BatchedQueueRef::output_from_bytes(&data);
    assert_zerocopy_error(result, "Insufficient size should fail");

    // Test 3: Wrong queue type
    let (data, _pubkey) = QueueAccountBuilder::output_queue()
        .build_with_wrong_queue_type(999);
    let result = BatchedQueueRef::output_from_bytes(&data);
    assert_metadata_error(
        result,
        MerkleTreeMetadataError::InvalidQueueType,
        "Wrong queue type should fail",
    );
}

#[test]
fn test_queue_ref_check_association_success() {
    let associated_tree = Pubkey::new_unique();
    let (data, _pubkey) = QueueAccountBuilder::output_queue()
        .with_associated_tree(associated_tree)
        .build();

    let queue_ref = BatchedQueueRef::output_from_bytes(&data).unwrap();

    // Check with correct associated_merkle_tree pubkey should succeed
    queue_ref
        .check_is_associated(&associated_tree)
        .expect("Association check should pass with correct pubkey");
}

#[test]
fn test_queue_ref_check_association_failure() {
    let associated_tree = Pubkey::new_unique();
    let wrong_tree = Pubkey::new_unique();
    let (data, _pubkey) = QueueAccountBuilder::output_queue()
        .with_associated_tree(associated_tree)
        .build();

    let queue_ref = BatchedQueueRef::output_from_bytes(&data).unwrap();

    // Check with wrong pubkey should fail
    let result = queue_ref.check_is_associated(&wrong_tree);
    assert_metadata_error(
        result,
        MerkleTreeMetadataError::MerkleTreeAndQueueNotAssociated,
        "Association check should fail with wrong pubkey",
    );
}

#[test]
fn test_queue_ref_prove_inclusion_by_index() {
    let (mut account_data, _pubkey) = QueueAccountBuilder::output_queue()
        .with_batch_size(10)
        .with_zkp_batch_size(2)
        .build();

    // Insert test values via the proper insertion API
    let test_hash_1 = [0x11; 32];
    let test_hash_2 = [0x22; 32];
    {
        let mut queue_mut = BatchedQueueAccount::output_from_bytes(&mut account_data).unwrap();
        queue_mut.insert_into_current_batch(&test_hash_1, &0).unwrap();
        queue_mut.insert_into_current_batch(&test_hash_2, &0).unwrap();
    }

    // Read via immutable ref
    let queue_ref = BatchedQueueRef::output_from_bytes(&account_data).unwrap();

    // Table-driven test cases
    struct TestCase {
        name: &'static str,
        leaf_index: u64,
        hash: [u8; 32],
        expected: Result<bool, &'static str>,
    }

    let test_cases = vec![
        TestCase {
            name: "Valid index with matching hash",
            leaf_index: 0,
            hash: test_hash_1,
            expected: Ok(true),
        },
        TestCase {
            name: "Valid index with wrong hash",
            leaf_index: 0,
            hash: [0xFF; 32],
            expected: Err("InclusionProofByIndexFailed"),
        },
        TestCase {
            name: "Valid index with matching hash (second element)",
            leaf_index: 1,
            hash: test_hash_2,
            expected: Ok(true),
        },
        TestCase {
            name: "Index beyond next_index",
            leaf_index: 100,
            hash: [0u8; 32],
            expected: Err("InvalidIndex"),
        },
    ];

    for tc in test_cases {
        let result = queue_ref.prove_inclusion_by_index(tc.leaf_index, &tc.hash);
        match tc.expected {
            Ok(expected_bool) => {
                assert_eq!(
                    result.expect(&format!("{} should succeed", tc.name)),
                    expected_bool,
                    "{}",
                    tc.name
                );
            }
            Err(expected_error) => {
                assert!(
                    result.is_err(),
                    "{} should fail with {}",
                    tc.name,
                    expected_error
                );
            }
        }
    }
}

#[test]
fn test_queue_ref_prove_inclusion_empty_queue() {
    let (account_data, _pubkey) = QueueAccountBuilder::output_queue().build();

    let queue_ref = BatchedQueueRef::output_from_bytes(&account_data).unwrap();

    // Any index on empty queue should return InvalidIndex error
    let result = queue_ref.prove_inclusion_by_index(0, &[0u8; 32]);
    assert_error(
        result,
        light_batched_merkle_tree::errors::BatchedMerkleTreeError::InvalidIndex,
        "Empty queue should return InvalidIndex",
    );
}

#[test]
fn test_queue_ref_metadata_deref() {
    let associated_tree = Pubkey::new_unique();
    let (data, _pubkey) = QueueAccountBuilder::output_queue()
        .with_associated_tree(associated_tree)
        .with_batch_size(8)
        .with_zkp_batch_size(4)
        .build();

    let queue_ref = BatchedQueueRef::output_from_bytes(&data).unwrap();

    // Access metadata fields via Deref
    assert_eq!(
        queue_ref.metadata.associated_merkle_tree,
        associated_tree,
        "associated_merkle_tree should be accessible via Deref"
    );
    assert_eq!(
        queue_ref.metadata.queue_type,
        OUTPUT_STATE_QUEUE_TYPE_V2,
        "queue_type should be accessible via Deref"
    );

    // Access batch_metadata fields
    assert_eq!(
        queue_ref.batch_metadata.batch_size,
        8,
        "batch_size should be accessible"
    );
    assert_eq!(
        queue_ref.batch_metadata.zkp_batch_size,
        4,
        "zkp_batch_size should be accessible"
    );
}

#[test]
fn test_queue_ref_pubkey() {
    let (data, _expected_pubkey) = QueueAccountBuilder::output_queue().build();
    let queue_ref = BatchedQueueRef::output_from_bytes(&data).unwrap();

    // pubkey() should return the default pubkey (since output_from_bytes uses Pubkey::default())
    // Note: The builder creates the account with a unique pubkey but output_from_bytes
    // passes Pubkey::default() in the from_bytes call
    assert_eq!(
        *queue_ref.pubkey(),
        Pubkey::default(),
        "pubkey() should return default pubkey"
    );
}

#[test]
fn test_queue_ref_different_batch_configurations() {
    // Table-driven test: different batch configurations
    struct TestConfig {
        name: &'static str,
        batch_size: u64,
        zkp_batch_size: u64,
        tree_capacity: u64,
    }

    let configs = vec![
        TestConfig {
            name: "Small batches",
            batch_size: 2,
            zkp_batch_size: 1,
            tree_capacity: 8,
        },
        TestConfig {
            name: "Medium batches",
            batch_size: 10,
            zkp_batch_size: 5,
            tree_capacity: 64,
        },
        TestConfig {
            name: "Large batches",
            batch_size: 100,
            zkp_batch_size: 10,
            tree_capacity: 1024,
        },
    ];

    for config in configs {
        let (data, _pubkey) = QueueAccountBuilder::output_queue()
            .with_batch_size(config.batch_size)
            .with_zkp_batch_size(config.zkp_batch_size)
            .with_tree_capacity(config.tree_capacity)
            .build();

        let queue_ref = BatchedQueueRef::output_from_bytes(&data)
            .unwrap_or_else(|_| panic!("{}: Failed to deserialize", config.name));

        // Verify configuration is preserved
        assert_eq!(
            queue_ref.batch_metadata.batch_size, config.batch_size,
            "{}: Batch size mismatch",
            config.name
        );
        assert_eq!(
            queue_ref.batch_metadata.zkp_batch_size, config.zkp_batch_size,
            "{}: ZKP batch size mismatch",
            config.name
        );
        assert_eq!(
            queue_ref.tree_capacity, config.tree_capacity,
            "{}: Tree capacity mismatch",
            config.name
        );
    }
}
