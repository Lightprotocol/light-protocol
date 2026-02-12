mod test_helpers;

use light_batched_merkle_tree::{
    merkle_tree::BatchedMerkleTreeAccount, merkle_tree_ref::BatchedMerkleTreeRef,
};
use light_compressed_account::{pubkey::Pubkey, TreeType, STATE_MERKLE_TREE_TYPE_V2};
use light_merkle_tree_metadata::{errors::MerkleTreeMetadataError, merkle_tree::MerkleTreeMetadata};
use test_helpers::{account_builders::MerkleTreeAccountBuilder, assertions::*};

#[test]
fn test_merkle_tree_ref_matches_mutable() {
    let mut account_data = vec![0u8; 3376];
    let batch_size = 5;
    let zkp_batch_size = 1;
    let root_history_len = 10;
    let num_iter = 1;
    let bloom_filter_capacity = 8000;
    let height = 40;
    let pubkey = Pubkey::new_unique();

    // Initialize via mutable path.
    let _account = BatchedMerkleTreeAccount::init(
        &mut account_data,
        &pubkey,
        MerkleTreeMetadata::default(),
        root_history_len,
        batch_size,
        zkp_batch_size,
        height,
        num_iter,
        bloom_filter_capacity,
        TreeType::AddressV2,
    )
    .unwrap();

    // Collect expected values from the mutable path.
    let expected_metadata;
    let expected_height;
    let expected_tree_type;
    let expected_seq;
    let expected_pubkey;
    let mut expected_roots = Vec::new();
    let expected_bf0;
    let expected_bf1;
    {
        let tree_mut =
            BatchedMerkleTreeAccount::address_from_bytes(&mut account_data, &pubkey).unwrap();
        expected_metadata = *tree_mut.get_metadata();
        expected_height = tree_mut.height;
        expected_tree_type = tree_mut.tree_type;
        expected_seq = tree_mut.sequence_number;
        expected_pubkey = *tree_mut.pubkey();
        for i in 0..root_history_len as usize {
            expected_roots.push(tree_mut.get_root_by_index(i).copied());
        }
        expected_bf0 = tree_mut.bloom_filter_stores[0].to_vec();
        expected_bf1 = tree_mut.bloom_filter_stores[1].to_vec();
    }

    // Read via immutable ref.
    let tree_ref = BatchedMerkleTreeRef::address_from_bytes(&account_data, &pubkey).unwrap();

    // Metadata should match (use Deref trait).
    assert_eq!(*tree_ref, expected_metadata);
    assert_eq!(tree_ref.height, expected_height);
    assert_eq!(tree_ref.tree_type, expected_tree_type);
    assert_eq!(tree_ref.sequence_number, expected_seq);
    assert_eq!(*tree_ref.pubkey(), expected_pubkey);

    // Root history should match (using root_history() accessor).
    for (i, expected) in expected_roots.iter().enumerate() {
        assert_eq!(Some(tree_ref.root_history()[i]), *expected);
    }

    // Bloom filter stores should match.
    assert_eq!(tree_ref.bloom_filter_stores[0], expected_bf0.as_slice());
    assert_eq!(tree_ref.bloom_filter_stores[1], expected_bf1.as_slice());

    // Non-inclusion check should work.
    let random_value = [42u8; 32];
    tree_ref
        .check_input_queue_non_inclusion(&random_value)
        .unwrap();
}

// ============================================================================
// New comprehensive tests for 100% coverage
// ============================================================================

#[test]
fn test_merkle_tree_ref_deserialization_matrix() {
    // Test matrix: tree type × API method (table-driven test)
    let test_cases = vec![
        ("State tree with state API", TreeType::StateV2, "state", true),
        ("Address tree with address API", TreeType::AddressV2, "address", true),
        ("State tree with address API", TreeType::StateV2, "address", false),
        ("Address tree with state API", TreeType::AddressV2, "state", false),
    ];

    for (description, tree_type, api, should_succeed) in test_cases {
        let (data, pubkey) = MerkleTreeAccountBuilder::state_tree()
            .with_tree_type(tree_type)
            .build();

        let result = if api == "state" {
            BatchedMerkleTreeRef::state_from_bytes(&data, &pubkey)
        } else {
            BatchedMerkleTreeRef::address_from_bytes(&data, &pubkey)
        };

        if should_succeed {
            assert!(
                result.is_ok(),
                "{}: Expected success but got error {:?}",
                description,
                result.err()
            );
            let tree_ref = result.unwrap();
            assert_eq!(
                *tree_ref.pubkey(),
                pubkey,
                "{}: Pubkey mismatch",
                description
            );
        } else {
            assert_metadata_error(
                result,
                MerkleTreeMetadataError::InvalidTreeType,
                description,
            );
        }
    }
}

#[test]
fn test_merkle_tree_ref_from_bytes_errors() {
    // Test 1: Bad discriminator
    let (data, pubkey) = MerkleTreeAccountBuilder::state_tree()
        .build_with_bad_discriminator();
    let result = BatchedMerkleTreeRef::state_from_bytes(&data, &pubkey);
    assert_account_error(result, "Bad discriminator should fail");

    // Test 2: Insufficient size - truncate to just past discriminator so metadata parse fails
    let (data, pubkey) = MerkleTreeAccountBuilder::state_tree().build();
    let truncated = &data[..16]; // 8 bytes discriminator + 8 bytes (not enough for metadata)
    let result = BatchedMerkleTreeRef::state_from_bytes(truncated, &pubkey);
    assert_zerocopy_error(result, "Insufficient size should fail");

    // Test 3: Empty data (too small even for discriminator)
    let empty_data: &[u8] = &[0u8; 4];
    let result = BatchedMerkleTreeRef::state_from_bytes(empty_data, &pubkey);
    assert_account_error(result, "Empty data should fail discriminator check");

    // Test 4: Wrong tree type
    let (data, pubkey) = MerkleTreeAccountBuilder::state_tree()
        .build_with_wrong_tree_type(999);
    let result = BatchedMerkleTreeRef::state_from_bytes(&data, &pubkey);
    assert_metadata_error(
        result,
        MerkleTreeMetadataError::InvalidTreeType,
        "Wrong tree type should fail",
    );
}

#[test]
fn test_merkle_tree_ref_root_history_access() {
    let (mut account_data, pubkey) = MerkleTreeAccountBuilder::state_tree().build();
    let root_history_len = 10;

    // Populate root history via mutable ref and collect expected values
    let mut expected_roots = Vec::new();
    {
        let mut tree_mut = BatchedMerkleTreeAccount::state_from_bytes(&mut account_data, &pubkey).unwrap();
        // Init already pushed an initial root at index 0. Push additional values.
        for i in 1u8..6 {
            tree_mut.root_history.push([i; 32]);
        }
        // Collect expected root values from mutable path
        for i in 0..root_history_len as usize {
            expected_roots.push(tree_mut.get_root_by_index(i).copied());
        }
    }

    // Access via immutable ref
    let tree_ref = BatchedMerkleTreeRef::state_from_bytes(&account_data, &pubkey).unwrap();

    // Verify root_history at physical indices matches mutable path
    for (i, expected) in expected_roots.iter().enumerate() {
        assert_eq!(
            Some(tree_ref.root_history()[i]),
            *expected,
            "Root at index {} should match",
            i
        );
    }
}

#[test]
fn test_merkle_tree_ref_root_history_boundaries() {
    let (mut account_data, pubkey) = MerkleTreeAccountBuilder::state_tree().build();
    let root_history_len = 10;

    // Collect boundary values from the mutable path
    let first_root;
    let last_root;
    {
        let tree_mut = BatchedMerkleTreeAccount::state_from_bytes(&mut account_data, &pubkey).unwrap();
        // Init already pushed an initial root. Collect it at index 0.
        first_root = tree_mut.get_root_by_index(0).copied().unwrap();
        last_root = tree_mut.get_root_by_index((root_history_len - 1) as usize).copied().unwrap();
    }

    let tree_ref = BatchedMerkleTreeRef::state_from_bytes(&account_data, &pubkey).unwrap();

    // Verify boundary access works (index 0 and capacity-1)
    assert_eq!(tree_ref.root_history()[0], first_root, "First root should be accessible");
    assert_eq!(
        tree_ref.root_history()[(root_history_len - 1) as usize],
        last_root,
        "Last root should be accessible"
    );
}

#[test]
#[should_panic(expected = "index out of bounds")]
fn test_merkle_tree_ref_root_history_out_of_bounds() {
    let (data, pubkey) = MerkleTreeAccountBuilder::state_tree()
        .with_root_history_capacity(5)
        .build();

    let tree_ref = BatchedMerkleTreeRef::state_from_bytes(&data, &pubkey).unwrap();

    // Access beyond capacity should panic
    let _ = tree_ref.root_history()[10]; // Capacity is 5, so index 10 is out of bounds
}

#[test]
fn test_merkle_tree_ref_bloom_filter_stores() {
    let (data, pubkey) = MerkleTreeAccountBuilder::state_tree()
        .with_bloom_filter_capacity(8000)
        .build();

    let tree_ref = BatchedMerkleTreeRef::state_from_bytes(&data, &pubkey).unwrap();

    // Verify bloom filter stores are accessible
    assert_eq!(
        tree_ref.bloom_filter_stores.len(),
        2,
        "Should have 2 bloom filter stores"
    );

    // Each bloom filter store should have the correct size (capacity / 8)
    let expected_size = 8000 / 8;
    assert_eq!(
        tree_ref.bloom_filter_stores[0].len(),
        expected_size,
        "First bloom filter store should have correct size"
    );
    assert_eq!(
        tree_ref.bloom_filter_stores[1].len(),
        expected_size,
        "Second bloom filter store should have correct size"
    );
}

#[test]
fn test_merkle_tree_ref_check_non_inclusion_empty() {
    // Table-driven test: various values against empty bloom filters
    let test_values = vec![
        [0u8; 32],
        [0xFF; 32],
        [0x55; 32],
        [0xAA; 32],
    ];

    let (data, pubkey) = MerkleTreeAccountBuilder::state_tree().build();
    let tree_ref = BatchedMerkleTreeRef::state_from_bytes(&data, &pubkey).unwrap();

    // All should pass non-inclusion check (no values inserted yet)
    for (i, value) in test_values.iter().enumerate() {
        tree_ref
            .check_input_queue_non_inclusion(value)
            .unwrap_or_else(|_| panic!("Test value {} should pass non-inclusion on empty filter", i));
    }
}

#[test]
fn test_merkle_tree_ref_check_non_inclusion_with_values() {
    let (mut account_data, pubkey) = MerkleTreeAccountBuilder::state_tree()
        .with_num_iters(3)
        .build();

    let inserted_value = [0x42; 32];
    let non_inserted_value = [0x99; 32];

    // Insert value into bloom filter via mutable path
    {
        let mut tree_mut = BatchedMerkleTreeAccount::state_from_bytes(&mut account_data, &pubkey).unwrap();
        // Insert into current batch's bloom filter
        use light_bloom_filter::BloomFilter;
        // Get metadata values first before borrowing bloom_filter_stores
        let num_iters = tree_mut.queue_batches.batches[0].num_iters as usize;
        let capacity = tree_mut.queue_batches.batches[0].bloom_filter_capacity;
        let bloom_filter = &mut tree_mut.bloom_filter_stores[0];
        let mut bf = BloomFilter::new(num_iters, capacity, bloom_filter).unwrap();
        bf.insert(&inserted_value).unwrap();
    }

    // Read via immutable ref
    let tree_ref = BatchedMerkleTreeRef::state_from_bytes(&account_data, &pubkey).unwrap();

    // Check inserted value fails non-inclusion
    assert!(
        tree_ref.check_input_queue_non_inclusion(&inserted_value).is_err(),
        "Inserted value should fail non-inclusion check"
    );

    // Check non-inserted value passes non-inclusion
    assert!(
        tree_ref.check_input_queue_non_inclusion(&non_inserted_value).is_ok(),
        "Non-inserted value should pass non-inclusion check"
    );
}

#[test]
fn test_merkle_tree_ref_metadata_deref() {
    let (data, pubkey) = MerkleTreeAccountBuilder::state_tree()
        .with_height(26)
        .build();

    let tree_ref = BatchedMerkleTreeRef::state_from_bytes(&data, &pubkey).unwrap();

    // Access metadata fields via Deref
    assert_eq!(
        tree_ref.tree_type,
        STATE_MERKLE_TREE_TYPE_V2,
        "tree_type should be accessible via Deref"
    );
    assert_eq!(
        tree_ref.height,
        26,
        "height should be accessible via Deref"
    );
    assert_eq!(
        tree_ref.sequence_number,
        0,
        "sequence_number should be accessible via Deref"
    );

    // Access queue_batches fields
    assert_eq!(
        tree_ref.queue_batches.num_batches,
        2,
        "num_batches should be accessible"
    );
    assert_eq!(
        tree_ref.queue_batches.batch_size,
        5,
        "batch_size should be accessible"
    );
}

#[test]
fn test_merkle_tree_ref_different_configurations() {
    // Table-driven test: different tree configurations
    struct TestConfig {
        name: &'static str,
        batch_size: u64,
        zkp_batch_size: u64,
        root_history_capacity: u32,
        height: u32,
        bloom_filter_capacity: u64,
    }

    let configs = vec![
        TestConfig {
            name: "Minimal config",
            batch_size: 2,
            zkp_batch_size: 1,
            root_history_capacity: 2,
            height: 10,
            bloom_filter_capacity: 1024, // Must be multiple of 64 for alignment
        },
        TestConfig {
            name: "Default config",
            batch_size: 5,
            zkp_batch_size: 1,
            root_history_capacity: 10,
            height: 40,
            bloom_filter_capacity: 8000,
        },
        TestConfig {
            name: "Large config",
            batch_size: 100,
            zkp_batch_size: 10,
            root_history_capacity: 100,
            height: 26,
            bloom_filter_capacity: 16000, // Must be multiple of 64 for alignment
        },
    ];

    for config in configs {
        let (data, pubkey) = MerkleTreeAccountBuilder::state_tree()
            .with_batch_size(config.batch_size)
            .with_zkp_batch_size(config.zkp_batch_size)
            .with_root_history_capacity(config.root_history_capacity)
            .with_height(config.height)
            .with_bloom_filter_capacity(config.bloom_filter_capacity)
            .build();

        let tree_ref = BatchedMerkleTreeRef::state_from_bytes(&data, &pubkey)
            .unwrap_or_else(|_| panic!("{}: Failed to deserialize", config.name));

        // Verify configuration is preserved
        assert_eq!(
            tree_ref.height, config.height,
            "{}: Height mismatch",
            config.name
        );
        assert_eq!(
            tree_ref.queue_batches.batch_size, config.batch_size,
            "{}: Batch size mismatch",
            config.name
        );
        assert_eq!(
            tree_ref.queue_batches.zkp_batch_size, config.zkp_batch_size,
            "{}: ZKP batch size mismatch",
            config.name
        );
        assert_eq!(
            tree_ref.queue_batches.bloom_filter_capacity, config.bloom_filter_capacity,
            "{}: Bloom filter capacity mismatch",
            config.name
        );
    }
}
