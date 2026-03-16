mod test_helpers;

use light_batched_merkle_tree::{queue::BatchedQueueAccount, queue_ref::BatchedQueueRef};
use light_compressed_account::pubkey::Pubkey;
use light_merkle_tree_metadata::errors::MerkleTreeMetadataError;
use test_helpers::{account_builders::QueueAccountBuilder, assertions::*};

#[test]
fn test_queue_ref_deserialization_errors() {
    // Test 1: Bad discriminator
    let (data, _pubkey) = QueueAccountBuilder::output_queue().build_with_bad_discriminator();
    let result = BatchedQueueRef::output_from_bytes(&data);
    assert_account_error(result, "Bad discriminator should fail");

    // Test 2: Insufficient size
    let (data, _pubkey) = QueueAccountBuilder::output_queue().build_too_small();
    let result = BatchedQueueRef::output_from_bytes(&data);
    assert_zerocopy_error(result, "Insufficient size should fail");

    // Test 3: Wrong queue type
    let (data, _pubkey) = QueueAccountBuilder::output_queue().build_with_wrong_queue_type(999);
    let result = BatchedQueueRef::output_from_bytes(&data);
    assert_metadata_error(
        result,
        MerkleTreeMetadataError::InvalidQueueType,
        "Wrong queue type should fail",
    );

    // Test 4: Wrong pubkey association
    let associated_tree = Pubkey::new_unique();
    let wrong_tree = Pubkey::new_unique();
    let (data, _pubkey) = QueueAccountBuilder::output_queue()
        .with_associated_tree(associated_tree)
        .build();
    let queue_ref = BatchedQueueRef::output_from_bytes(&data).unwrap();
    assert_metadata_error(
        queue_ref.check_is_associated(&wrong_tree),
        MerkleTreeMetadataError::MerkleTreeAndQueueNotAssociated,
        "Association check should fail with wrong pubkey",
    );

    // Test 5: Empty queue prove_inclusion returns InvalidIndex
    let (data, _pubkey) = QueueAccountBuilder::output_queue().build();
    let queue_ref = BatchedQueueRef::output_from_bytes(&data).unwrap();
    assert_error(
        queue_ref.prove_inclusion_by_index(0, &[0u8; 32]),
        light_batched_merkle_tree::errors::BatchedMerkleTreeError::InvalidIndex,
        "Empty queue should return InvalidIndex",
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
        queue_mut
            .insert_into_current_batch(&test_hash_1, &0)
            .unwrap();
        queue_mut
            .insert_into_current_batch(&test_hash_2, &0)
            .unwrap();
    }

    // Read via immutable ref
    let queue_ref = BatchedQueueRef::output_from_bytes(&account_data).unwrap();

    // Valid index with matching hash
    assert_eq!(
        queue_ref.prove_inclusion_by_index(0, &test_hash_1).unwrap(),
        true,
        "Valid index with matching hash should return true"
    );
    assert_eq!(
        queue_ref.prove_inclusion_by_index(1, &test_hash_2).unwrap(),
        true,
        "Second element with matching hash should return true"
    );

    // Valid index with wrong hash returns error
    assert!(
        queue_ref.prove_inclusion_by_index(0, &[0xFF; 32]).is_err(),
        "Wrong hash should return error"
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

#[test]
fn test_queue_ref_randomized_equivalence() {
    use rand::{rngs::StdRng, Rng, SeedableRng};

    let mut rng = StdRng::seed_from_u64(0xCAFE_BABE);
    let batch_size = 1000u64;
    let associated_tree = Pubkey::new_unique();

    let (mut account_data, _pubkey) = QueueAccountBuilder::output_queue()
        .with_batch_size(batch_size)
        .with_zkp_batch_size(1)
        .with_associated_tree(associated_tree)
        .build();

    let mut inserted: Vec<(u64, [u8; 32])> = Vec::new();

    for _ in 0..1000 {
        // Insert a value into the current batch (stop when batch is full).
        let value: [u8; 32] = rng.gen();
        let slot = 0u64;
        {
            let mut queue_mut = BatchedQueueAccount::output_from_bytes(&mut account_data).unwrap();
            let result = queue_mut.insert_into_current_batch(&value, &slot);
            if result.is_ok() {
                inserted.push((inserted.len() as u64, value));
            } else {
                // Batch is full, skip further inserts.
                continue;
            }
        }

        // Clone data so we can deserialize both paths independently.
        let mut account_data_clone = account_data.clone();

        let queue_ref = BatchedQueueRef::output_from_bytes(&account_data).unwrap();
        let queue_mut = BatchedQueueAccount::output_from_bytes(&mut account_data_clone).unwrap();

        // Metadata via Deref.
        assert_eq!(*queue_ref, *queue_mut.get_metadata());

        // next_index.
        assert_eq!(
            queue_ref.batch_metadata.next_index,
            queue_mut.get_metadata().batch_metadata.next_index,
        );

        // Prove inclusion for all inserted values.
        for &(leaf_index, ref val) in &inserted {
            assert_eq!(
                queue_ref.prove_inclusion_by_index(leaf_index, val).unwrap(),
                true,
                "Inclusion failed at leaf_index {}",
                leaf_index
            );
        }

        // Association check.
        queue_ref.check_is_associated(&associated_tree).unwrap();

        // Pubkey accessor.
        assert_eq!(*queue_ref.pubkey(), *queue_mut.pubkey());
    }
}
