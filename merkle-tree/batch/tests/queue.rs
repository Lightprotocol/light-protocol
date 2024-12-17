use light_batched_merkle_tree::{
    batch_metadata::BatchMetadata,
    errors::BatchedMerkleTreeError,
    queue::{
        assert_queue_zero_copy_inited, queue_account_size, BatchedQueueAccount,
        ZeroCopyBatchedQueueAccount,
    },
};
use light_merkle_tree_metadata::{
    access::AccessMetadata,
    queue::{QueueMetadata, QueueType},
    rollover::RolloverMetadata,
};
use solana_program::pubkey::Pubkey;

pub fn get_test_account_and_account_data(
    batch_size: u64,
    num_batches: u64,
    queue_type: QueueType,
    bloom_filter_capacity: u64,
) -> (BatchedQueueAccount, Vec<u8>) {
    let metadata = QueueMetadata {
        next_queue: Pubkey::new_unique(),
        access_metadata: AccessMetadata::default(),
        rollover_metadata: RolloverMetadata::default(),
        queue_type: queue_type as u64,
        associated_merkle_tree: Pubkey::new_unique(),
    };

    let account = BatchedQueueAccount {
        metadata: metadata.clone(),
        next_index: 0,
        queue: BatchMetadata {
            batch_size: batch_size as u64,
            num_batches: num_batches as u64,
            currently_processing_batch_index: 0,
            next_full_batch_index: 0,
            bloom_filter_capacity,
            zkp_batch_size: 10,
        },
    };
    let account_data: Vec<u8> =
        vec![0; queue_account_size(&account.queue, account.metadata.queue_type).unwrap()];
    (account, account_data)
}

#[test]
fn test_output_queue_account() {
    let batch_size = 100;
    // 1 batch in progress, 1 batch ready to be processed
    let num_batches = 2;
    let bloom_filter_capacity = 0;
    let bloom_filter_num_iters = 0;
    for queue_type in vec![QueueType::Output] {
        let (ref_account, mut account_data) = get_test_account_and_account_data(
            batch_size,
            num_batches,
            queue_type,
            bloom_filter_capacity,
        );
        ZeroCopyBatchedQueueAccount::init(
            ref_account.metadata,
            num_batches,
            batch_size,
            10,
            &mut account_data,
            bloom_filter_num_iters,
            bloom_filter_capacity,
        )
        .unwrap();

        assert_queue_zero_copy_inited(&mut account_data, ref_account, bloom_filter_num_iters);
        let mut zero_copy_account =
            ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut account_data).unwrap();
        let value = [1u8; 32];
        zero_copy_account.insert_into_current_batch(&value).unwrap();
        // assert!(zero_copy_account.insert_into_current_batch(&value).is_ok());
        if queue_type != QueueType::Output {
            assert!(zero_copy_account.insert_into_current_batch(&value).is_err());
        }
    }
}

#[test]
fn test_value_exists_in_value_vec_present() {
    let (account, mut account_data) =
        get_test_account_and_account_data(100, 2, QueueType::Output, 0);
    let mut zero_copy_account = ZeroCopyBatchedQueueAccount::init(
        account.metadata.clone(),
        2,
        100,
        10,
        &mut account_data,
        0,
        0,
    )
    .unwrap();

    let value = [1u8; 32];
    let value2 = [2u8; 32];

    // 1. Functional for 1 value
    {
        zero_copy_account.insert_into_current_batch(&value).unwrap();
        assert_eq!(
            zero_copy_account.prove_inclusion_by_index(1, &value),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
        assert_eq!(
            zero_copy_account.prove_inclusion_by_index_and_zero_out_leaf(1, &value),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
        assert_eq!(
            zero_copy_account.prove_inclusion_by_index_and_zero_out_leaf(0, &value2),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
        assert!(zero_copy_account
            .prove_inclusion_by_index(0, &value)
            .is_ok());
        // prove inclusion for value out of range returns false
        assert_eq!(
            zero_copy_account
                .prove_inclusion_by_index(100000, &[0u8; 32])
                .unwrap(),
            false
        );
        assert!(zero_copy_account
            .prove_inclusion_by_index_and_zero_out_leaf(0, &value)
            .is_ok());
    }
    // 2. Functional does not succeed on second invocation
    {
        assert_eq!(
            zero_copy_account.prove_inclusion_by_index_and_zero_out_leaf(0, &value),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
        assert_eq!(
            zero_copy_account.prove_inclusion_by_index(0, &value),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
    }

    // 3. Functional for 2 values
    {
        zero_copy_account
            .insert_into_current_batch(&value2)
            .unwrap();

        assert_eq!(
            zero_copy_account.prove_inclusion_by_index_and_zero_out_leaf(0, &value2),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
        assert!(zero_copy_account
            .prove_inclusion_by_index_and_zero_out_leaf(1, &value2)
            .is_ok());
    }
    // 4. Functional does not succeed on second invocation
    {
        assert_eq!(
            zero_copy_account.prove_inclusion_by_index_and_zero_out_leaf(1, &value2),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
    }
}
