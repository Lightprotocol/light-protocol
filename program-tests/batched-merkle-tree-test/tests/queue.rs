use light_batched_merkle_tree::{
    batch::Batch,
    constants::NUM_BATCHES,
    errors::BatchedMerkleTreeError,
    queue::{test_utils::assert_queue_zero_copy_inited, BatchedQueueAccount, BatchedQueueMetadata},
    queue_batch_metadata::QueueBatches,
};
use light_compressed_account::{pubkey::Pubkey, QueueType};
use light_merkle_tree_metadata::{
    access::AccessMetadata, queue::QueueMetadata, rollover::RolloverMetadata,
};

pub fn get_test_account_and_account_data(
    batch_size: u64,
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
        batch_metadata: QueueBatches {
            batch_size,
            num_batches: NUM_BATCHES as u64,
            currently_processing_batch_index: 0,
            pending_batch_index: 0,
            bloom_filter_capacity,
            zkp_batch_size: 10,
            next_index: 0,
            batches: [
                Batch::new(0, 0, batch_size, 10, 0),
                Batch::new(0, 0, batch_size, 10, batch_size),
            ],
        },
        ..Default::default()
    };
    let account_data: Vec<u8> = vec![
        0;
        account
            .batch_metadata
            .queue_account_size(account.metadata.queue_type)
            .unwrap()
    ];
    (account, account_data)
}

#[test]
fn test_output_queue_account() {
    let batch_size = 100;
    // 1 batch in progress, 1 batch ready to be processed
    let bloom_filter_capacity = 0;
    let bloom_filter_num_iters = 0;
    let queue_pubkey = Pubkey::new_unique();
    let current_slot = 123;
    {
        let queue_type = QueueType::OutputStateV2;
        let (ref_account, mut account_data) =
            get_test_account_and_account_data(batch_size, queue_type, bloom_filter_capacity);
        BatchedQueueAccount::init(
            &mut account_data,
            ref_account.metadata,
            batch_size,
            10,
            bloom_filter_num_iters,
            bloom_filter_capacity,
            queue_pubkey,
            1024, // 2^10 for test purposes
        )
        .unwrap();

        assert_queue_zero_copy_inited(&mut account_data, ref_account);
        let mut account = BatchedQueueAccount::output_from_bytes(&mut account_data).unwrap();
        let value = [1u8; 32];
        account
            .insert_into_current_batch(&value, &current_slot)
            .unwrap();
        let current_batch = account.batch_metadata.get_current_batch();
        assert_eq!(current_batch.get_num_inserted_elements(), 1);
        assert_eq!(current_batch.start_slot, current_slot);
    }
}

#[test]
fn test_value_exists_in_value_vec() {
    let (account, mut account_data) =
        get_test_account_and_account_data(100, QueueType::OutputStateV2, 0);
    let queue_pubkey = Pubkey::new_unique();
    let mut account = BatchedQueueAccount::init(
        &mut account_data,
        account.metadata,
        100,
        10,
        0,
        0,
        queue_pubkey,
        1024, // 2^10 for test purposes
    )
    .unwrap();
    let current_slot = 2;
    let value = [1u8; 32];
    let value2 = [2u8; 32];

    // 1. Functional for 1 value
    {
        account
            .insert_into_current_batch(&value, &current_slot)
            .unwrap();
        assert_eq!(
            account.prove_inclusion_by_index(1, &value),
            Err(BatchedMerkleTreeError::InvalidIndex),
            "With invalid index."
        );
        assert_eq!(
            account.prove_inclusion_by_index_and_zero_out_leaf(1, &value, false),
            Err(BatchedMerkleTreeError::InvalidIndex),
            "With invalid index."
        );
        assert_eq!(
            account.prove_inclusion_by_index_and_zero_out_leaf(1, &value, true),
            Err(BatchedMerkleTreeError::InvalidIndex),
            "With invalid index."
        );
        assert_eq!(
            account.prove_inclusion_by_index_and_zero_out_leaf(0, &value2, false),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed),
            "With invalid value."
        );
        assert_eq!(
            account.prove_inclusion_by_index_and_zero_out_leaf(0, &value2, true),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed),
            "With invalid value."
        );
        assert!(account.prove_inclusion_by_index(0, &value).is_ok());

        assert!(account
            .prove_inclusion_by_index_and_zero_out_leaf(0, &value, false)
            .is_ok());
        let current_batch = account.batch_metadata.get_current_batch();
        assert_eq!(current_batch.get_num_inserted_elements(), 1);
        assert_eq!(current_batch.start_slot, current_slot);
    }
    // 2. Functional does not succeed on second invocation
    {
        assert_eq!(
            account.prove_inclusion_by_index_and_zero_out_leaf(0, &value, false),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
        assert_eq!(
            account.prove_inclusion_by_index_and_zero_out_leaf(0, &value, true),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
        assert_eq!(
            account.prove_inclusion_by_index(0, &value),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
    }

    // 3. Functional for value 2 with proof by index enforced
    {
        account
            .insert_into_current_batch(&value2, &current_slot)
            .unwrap();

        assert_eq!(
            account.prove_inclusion_by_index_and_zero_out_leaf(0, &value2, false),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
        assert_eq!(
            account.prove_inclusion_by_index_and_zero_out_leaf(0, &value2, true),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
        assert!(account
            .prove_inclusion_by_index_and_zero_out_leaf(1, &value2, true)
            .is_ok());
    }
    // 4. Functional does not succeed on second invocation
    // regardless whether it is marked as proof by index
    {
        assert_eq!(
            account.prove_inclusion_by_index_and_zero_out_leaf(1, &value2, false),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
        assert_eq!(
            account.prove_inclusion_by_index_and_zero_out_leaf(1, &value2, true),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
        assert_eq!(
            account.prove_inclusion_by_index_and_zero_out_leaf(0, &value2, true),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
    }
    for i in 0..254 {
        if i == 197 {
            account.batch_metadata.batches[0]
                .advance_state_to_inserted()
                .unwrap();
        }
        let mut value = [0; 32];
        println!("i {}", i);
        value[31] = i;
        account
            .insert_into_current_batch(&value, &current_slot)
            .unwrap();
    }
    // Value out of range lower bound.
    {
        // prove inclusion for value out of range returns false
        assert!(!account.prove_inclusion_by_index(3, &[0u8; 32]).unwrap());
        let proof_by_index = true;
        assert_eq!(
            account.prove_inclusion_by_index_and_zero_out_leaf(4, &[0u8; 32], proof_by_index),
            Err(BatchedMerkleTreeError::InclusionProofByIndexFailed)
        );
        let proof_by_index = false;
        assert_eq!(
            account.prove_inclusion_by_index_and_zero_out_leaf(4, &[0u8; 32], proof_by_index),
            Ok(())
        );
    }
}
