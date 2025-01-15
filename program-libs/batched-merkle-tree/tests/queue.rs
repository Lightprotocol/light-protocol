use light_batched_merkle_tree::{
    batch_metadata::BatchMetadata,
    errors::BatchedMerkleTreeError,
    queue::{
        assert_queue_zero_copy_inited, queue_account_size, BatchedQueueAccount,
        BatchedQueueMetadata,
    },
};
use light_merkle_tree_metadata::{
    access::AccessMetadata,
    queue::{QueueMetadata, QueueType},
    rollover::RolloverMetadata,
};
use light_utils::pubkey::Pubkey;

pub fn get_test_account_and_account_data(
    batch_size: u64,
    num_batches: u64,
    queue_type: QueueType,
    bloom_filter_capacity: u64,
) -> (BatchedQueueMetadata, Vec<u8>) {
    let metadata = QueueMetadata {
        next_queue: Pubkey::new_unique(),
        access_metadata: AccessMetadata::default(),
        rollover_metadata: RolloverMetadata::default(),
        queue_type: queue_type as u64,
        associated_merkle_tree: Pubkey::new_unique(),
    };

    let account = BatchedQueueMetadata {
        metadata,
        next_index: 0,
        batch_metadata: BatchMetadata {
            batch_size,
            num_batches,
            currently_processing_batch_index: 0,
            next_full_batch_index: 0,
            bloom_filter_capacity,
            zkp_batch_size: 10,
        },
        ..Default::default()
    };
    let account_data: Vec<u8> =
        vec![0; queue_account_size(&account.batch_metadata, account.metadata.queue_type).unwrap()];
    (account, account_data)
}

#[test]
fn test_output_queue_account() {
    let batch_size = 100;
    // 1 batch in progress, 1 batch ready to be processed
    let num_batches = 2;
    let bloom_filter_capacity = 0;
    let bloom_filter_num_iters = 0;
    {
        let queue_type = QueueType::BatchedOutput;
        let (ref_account, mut account_data) = get_test_account_and_account_data(
            batch_size,
            num_batches,
            queue_type,
            bloom_filter_capacity,
        );
        BatchedQueueAccount::init(
            &mut account_data,
            ref_account.metadata,
            num_batches,
            batch_size,
            10,
            bloom_filter_num_iters,
            bloom_filter_capacity,
        )
        .unwrap();

        assert_queue_zero_copy_inited(&mut account_data, ref_account, bloom_filter_num_iters);
        let mut account = BatchedQueueAccount::output_from_bytes(&mut account_data).unwrap();
        let value = [1u8; 32];
        account.insert_into_current_batch(&value).unwrap();
        // assert!(account.insert_into_current_batch(&value).is_ok());
        if queue_type != QueueType::BatchedOutput {
            assert!(account.insert_into_current_batch(&value).is_err());
        }
    }
}

#[test]
fn test_value_exists_in_value_vec_present() {
    let (account, mut account_data) =
        get_test_account_and_account_data(100, 2, QueueType::BatchedOutput, 0);
    let mut account =
        BatchedQueueAccount::init(&mut account_data, account.metadata, 2, 100, 10, 0, 0).unwrap();

    let value = [1u8; 32];
    let value2 = [2u8; 32];

    // 1. Functional for 1 value
    {
        account.insert_into_current_batch(&value).unwrap();
        assert_eq!(
            account.prove_inclusion_by_index(1, &value),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
        assert_eq!(
            account.prove_inclusion_by_index_and_zero_out_leaf(1, &value),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
        assert_eq!(
            account.prove_inclusion_by_index_and_zero_out_leaf(0, &value2),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
        assert!(account.prove_inclusion_by_index(0, &value).is_ok());
        // prove inclusion for value out of range returns false
        assert!(!account
            .prove_inclusion_by_index(100000, &[0u8; 32])
            .unwrap());
        assert!(account
            .prove_inclusion_by_index_and_zero_out_leaf(0, &value)
            .is_ok());
    }
    // 2. Functional does not succeed on second invocation
    {
        assert_eq!(
            account.prove_inclusion_by_index_and_zero_out_leaf(0, &value),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
        assert_eq!(
            account.prove_inclusion_by_index(0, &value),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
    }

    // 3. Functional for 2 values
    {
        account.insert_into_current_batch(&value2).unwrap();

        assert_eq!(
            account.prove_inclusion_by_index_and_zero_out_leaf(0, &value2),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
        assert!(account
            .prove_inclusion_by_index_and_zero_out_leaf(1, &value2)
            .is_ok());
    }
    // 4. Functional does not succeed on second invocation
    {
        assert_eq!(
            account.prove_inclusion_by_index_and_zero_out_leaf(1, &value2),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
    }
}
