mod test_helpers;

use light_batched_merkle_tree::{
    merkle_tree::BatchedMerkleTreeAccount, merkle_tree_ref::BatchedMerkleTreeRef,
};
use light_compressed_account::{pubkey::Pubkey, TreeType};
use light_merkle_tree_metadata::errors::MerkleTreeMetadataError;
use test_helpers::{account_builders::MerkleTreeAccountBuilder, assertions::*};

#[test]
fn test_merkle_tree_ref_deserialization_matrix() {
    // Test matrix: tree type x API method (table-driven test)
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

    // Test 5: Root history out-of-bounds returns None
    let (data, pubkey) = MerkleTreeAccountBuilder::state_tree()
        .with_root_history_capacity(5)
        .build();
    let tree_ref = BatchedMerkleTreeRef::state_from_bytes(&data, &pubkey).unwrap();
    assert!(tree_ref.get_root_by_index(10).is_none());
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

#[test]
fn test_merkle_tree_ref_randomized_equivalence() {
    use light_bloom_filter::BloomFilter;
    use rand::{rngs::StdRng, Rng, SeedableRng};

    let mut rng = StdRng::seed_from_u64(0xDEAD_BEEF);
    let root_history_capacity: u32 = 10;
    let bloom_filter_capacity: u64 = 100_000;

    let (mut account_data, pubkey) = MerkleTreeAccountBuilder::state_tree()
        .with_root_history_capacity(root_history_capacity)
        .with_bloom_filter_capacity(bloom_filter_capacity)
        .with_num_iters(1)
        .build();

    for _ in 0..1000 {
        let action = rng.gen_range(0..3u8);
        match action {
            0 => {
                // Push random root.
                let mut tree_mut =
                    BatchedMerkleTreeAccount::state_from_bytes(&mut account_data, &pubkey)
                        .unwrap();
                tree_mut.root_history.push(rng.gen());
            }
            1 => {
                // Insert into bloom filter of a random batch.
                let mut tree_mut =
                    BatchedMerkleTreeAccount::state_from_bytes(&mut account_data, &pubkey)
                        .unwrap();
                let batch_idx = rng.gen_range(0..2usize);
                let num_iters =
                    tree_mut.queue_batches.batches[batch_idx].num_iters as usize;
                let capacity =
                    tree_mut.queue_batches.batches[batch_idx].bloom_filter_capacity;
                let value: [u8; 32] = rng.gen();
                let mut bf = BloomFilter::new(
                    num_iters,
                    capacity,
                    &mut tree_mut.bloom_filter_stores[batch_idx],
                )
                .unwrap();
                bf.insert(&value).unwrap();
            }
            2 => {
                // Increment sequence number.
                let mut tree_mut =
                    BatchedMerkleTreeAccount::state_from_bytes(&mut account_data, &pubkey)
                        .unwrap();
                tree_mut.sequence_number += 1;
            }
            _ => unreachable!(),
        }

        // Clone data so we can deserialize both paths independently.
        let mut account_data_clone = account_data.clone();

        let tree_ref =
            BatchedMerkleTreeRef::state_from_bytes(&account_data, &pubkey).unwrap();
        let tree_mut =
            BatchedMerkleTreeAccount::state_from_bytes(&mut account_data_clone, &pubkey)
                .unwrap();

        // Metadata via Deref.
        assert_eq!(*tree_ref, *tree_mut.get_metadata());

        // Root history.
        for i in 0..root_history_capacity as usize {
            assert_eq!(
                tree_ref.get_root_by_index(i).copied(),
                tree_mut.get_root_by_index(i).copied(),
                "Root mismatch at index {}",
                i
            );
        }

        // Bloom filter stores byte-equal.
        for j in 0..2 {
            assert_eq!(
                tree_ref.bloom_filter_stores[j],
                tree_mut.bloom_filter_stores[j].as_ref(),
                "Bloom filter store {} mismatch",
                j
            );
        }

        // Pubkey.
        assert_eq!(tree_ref.pubkey(), tree_mut.pubkey());
    }

    // Non-inclusion coverage: insert a known value and verify it fails non-inclusion,
    // while a non-inserted value passes.
    let inserted_value = [0x42; 32];
    let non_inserted_value = [0x99; 32];
    {
        let mut tree_mut =
            BatchedMerkleTreeAccount::state_from_bytes(&mut account_data, &pubkey).unwrap();
        let num_iters = tree_mut.queue_batches.batches[0].num_iters as usize;
        let capacity = tree_mut.queue_batches.batches[0].bloom_filter_capacity;
        let mut bf =
            BloomFilter::new(num_iters, capacity, &mut tree_mut.bloom_filter_stores[0]).unwrap();
        bf.insert(&inserted_value).unwrap();
    }
    let tree_ref = BatchedMerkleTreeRef::state_from_bytes(&account_data, &pubkey).unwrap();
    assert!(
        tree_ref
            .check_input_queue_non_inclusion(&inserted_value)
            .is_err(),
        "Inserted value should fail non-inclusion check"
    );
    assert!(
        tree_ref
            .check_input_queue_non_inclusion(&non_inserted_value)
            .is_ok(),
        "Non-inserted value should pass non-inclusion check"
    );
}
