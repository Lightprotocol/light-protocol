use light_batched_merkle_tree::{queue::BatchedQueueAccount, queue_ref::BatchedQueueRef};
use light_compressed_account::{pubkey::Pubkey, QueueType};
use light_merkle_tree_metadata::queue::QueueMetadata;

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
