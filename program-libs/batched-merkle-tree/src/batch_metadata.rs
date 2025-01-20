use light_merkle_tree_metadata::{errors::MerkleTreeMetadataError, queue::QueueType};
use light_zero_copy::vec::ZeroCopyVecU64;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::{
    batch::{Batch, BatchState},
    constants::NUM_BATCHES,
    errors::BatchedMerkleTreeError,
    queue::BatchedQueueMetadata,
    BorshDeserialize, BorshSerialize,
};

#[repr(C)]
#[derive(
    BorshDeserialize,
    BorshSerialize,
    Debug,
    PartialEq,
    Default,
    Clone,
    Copy,
    FromBytes,
    IntoBytes,
    KnownLayout,
    Immutable,
)]
pub struct BatchMetadata {
    /// Number of batches.
    pub num_batches: u64,
    /// Number of elements in a batch.
    pub batch_size: u64,
    /// Number of elements in a ZKP batch.
    /// A batch has one or more ZKP batches.
    pub zkp_batch_size: u64,
    /// Bloom filter capacity.
    pub bloom_filter_capacity: u64,
    /// Batch elements are currently inserted in.
    pub currently_processing_batch_index: u64,
    /// Next batch to be inserted into the tree.
    pub next_full_batch_index: u64,
    pub batches: [Batch; 2],
}

impl BatchMetadata {
    /// Returns the number of ZKP batches contained within a single regular batch.
    pub fn get_num_zkp_batches(&self) -> u64 {
        self.batch_size / self.zkp_batch_size
    }

    pub fn get_current_batch(&self) -> &Batch {
        &self.batches[self.currently_processing_batch_index as usize]
    }

    pub fn get_current_batch_mut(&mut self) -> &mut Batch {
        &mut self.batches[self.currently_processing_batch_index as usize]
    }

    /// Validates that the batch size is properly divisible by the ZKP batch size.
    fn validate_batch_sizes(
        batch_size: u64,
        zkp_batch_size: u64,
    ) -> Result<(), BatchedMerkleTreeError> {
        if batch_size % zkp_batch_size != 0 {
            return Err(BatchedMerkleTreeError::BatchSizeNotDivisibleByZkpBatchSize);
        }
        Ok(())
    }

    pub fn new_output_queue(
        batch_size: u64,
        zkp_batch_size: u64,
    ) -> Result<Self, BatchedMerkleTreeError> {
        Self::validate_batch_sizes(batch_size, zkp_batch_size)?;
        Ok(BatchMetadata {
            num_batches: NUM_BATCHES as u64,
            zkp_batch_size,
            batch_size,
            currently_processing_batch_index: 0,
            next_full_batch_index: 0,
            // Output queues don't use bloom filters.
            bloom_filter_capacity: 0,
            batches: [
                Batch::new(0, 0, batch_size, zkp_batch_size, 0),
                Batch::new(0, 0, batch_size, zkp_batch_size, batch_size),
            ],
        })
    }

    pub fn new_input_queue(
        batch_size: u64,
        bloom_filter_capacity: u64,
        zkp_batch_size: u64,
        num_iters: u64,
        start_index: u64,
    ) -> Result<Self, BatchedMerkleTreeError> {
        Self::validate_batch_sizes(batch_size, zkp_batch_size)?;

        Ok(BatchMetadata {
            num_batches: NUM_BATCHES as u64,
            zkp_batch_size,
            batch_size,
            currently_processing_batch_index: 0,
            next_full_batch_index: 0,
            bloom_filter_capacity,
            batches: [
                Batch::new(
                    num_iters,
                    bloom_filter_capacity,
                    batch_size,
                    zkp_batch_size,
                    start_index,
                ),
                Batch::new(
                    num_iters,
                    bloom_filter_capacity,
                    batch_size,
                    zkp_batch_size,
                    batch_size + start_index,
                ),
            ],
        })
    }

    /// Increment the next full batch index if current state is BatchState::Inserted.
    pub fn increment_next_full_batch_index_if_inserted(&mut self, state: BatchState) {
        if state == BatchState::Inserted {
            self.next_full_batch_index = (self.next_full_batch_index + 1) % self.num_batches;
        }
    }

    /// Increment the currently_processing_batch_index if current state is BatchState::Full.
    pub fn increment_currently_processing_batch_index_if_full(&mut self) {
        let state = self.get_current_batch().get_state();
        if state == BatchState::Full {
            self.currently_processing_batch_index =
                (self.currently_processing_batch_index + 1) % self.num_batches;
        }
    }

    pub fn init(
        &mut self,
        batch_size: u64,
        zkp_batch_size: u64,
    ) -> Result<(), BatchedMerkleTreeError> {
        // Check that batch size is divisible by zkp_batch_size.
        Self::validate_batch_sizes(batch_size, zkp_batch_size)?;
        self.num_batches = NUM_BATCHES as u64;
        self.batch_size = batch_size;
        self.zkp_batch_size = zkp_batch_size;
        Ok(())
    }

    pub fn get_size_parameters(
        &self,
        queue_type: u64,
    ) -> Result<(usize, usize, usize), MerkleTreeMetadataError> {
        let num_batches = self.num_batches as usize;
        // Input queues don't store values.
        let num_value_stores = if queue_type == QueueType::BatchedOutput as u64 {
            num_batches
        } else if queue_type == QueueType::BatchedInput as u64 {
            0
        } else {
            return Err(MerkleTreeMetadataError::InvalidQueueType);
        };
        // Output queues don't use bloom filters.
        let num_stores = if queue_type == QueueType::BatchedInput as u64 {
            num_batches
        } else if queue_type == QueueType::BatchedOutput as u64 && self.bloom_filter_capacity == 0 {
            0
        } else {
            return Err(MerkleTreeMetadataError::InvalidQueueType);
        };
        Ok((num_value_stores, num_stores, num_batches))
    }

    pub fn queue_account_size(&self, queue_type: u64) -> Result<usize, BatchedMerkleTreeError> {
        let (num_value_vec, num_bloom_filter_stores, num_hashchain_store) =
            self.get_size_parameters(queue_type)?;
        let account_size = if queue_type != QueueType::BatchedOutput as u64 {
            0
        } else {
            BatchedQueueMetadata::LEN
        };
        let value_vecs_size =
            ZeroCopyVecU64::<[u8; 32]>::required_size_for_capacity(self.batch_size) * num_value_vec;
        // Bloomfilter capacity is in bits.
        let bloom_filter_stores_size =
            (self.bloom_filter_capacity / 8) as usize * num_bloom_filter_stores;
        let hashchain_store_size =
            ZeroCopyVecU64::<[u8; 32]>::required_size_for_capacity(self.get_num_zkp_batches())
                * num_hashchain_store;
        let size = account_size + value_vecs_size + bloom_filter_stores_size + hashchain_store_size;
        Ok(size)
    }
}

#[test]
fn test_increment_next_full_batch_index_if_inserted() {
    let mut metadata = BatchMetadata::new_input_queue(10, 10, 10, 3, 0).unwrap();
    assert_eq!(metadata.next_full_batch_index, 0);
    // increment next full batch index
    metadata.increment_next_full_batch_index_if_inserted(BatchState::Inserted);
    assert_eq!(metadata.next_full_batch_index, 1);
    // increment next full batch index
    metadata.increment_next_full_batch_index_if_inserted(BatchState::Inserted);
    assert_eq!(metadata.next_full_batch_index, 0);
    // try incrementing next full batch index with state not inserted
    metadata.increment_next_full_batch_index_if_inserted(BatchState::Fill);
    assert_eq!(metadata.next_full_batch_index, 0);
    metadata.increment_next_full_batch_index_if_inserted(BatchState::Full);
    assert_eq!(metadata.next_full_batch_index, 0);
}

#[test]
fn test_increment_currently_processing_batch_index_if_full() {
    let mut metadata = BatchMetadata::new_input_queue(10, 10, 10, 3, 0).unwrap();
    assert_eq!(metadata.currently_processing_batch_index, 0);
    metadata
        .get_current_batch_mut()
        .advance_state_to_full()
        .unwrap();
    // increment currently_processing_batch_index
    metadata.increment_currently_processing_batch_index_if_full();
    assert_eq!(metadata.currently_processing_batch_index, 1);
    assert_eq!(metadata.next_full_batch_index, 0);
    metadata
        .get_current_batch_mut()
        .advance_state_to_full()
        .unwrap();
    // increment currently_processing_batch_index
    metadata.increment_currently_processing_batch_index_if_full();
    assert_eq!(metadata.currently_processing_batch_index, 0);
    metadata
        .get_current_batch_mut()
        .advance_state_to_inserted()
        .unwrap();
    // try incrementing next full batch index with state not full
    metadata.increment_currently_processing_batch_index_if_full();
    assert_eq!(metadata.currently_processing_batch_index, 0);
    metadata
        .get_current_batch_mut()
        .advance_state_to_fill()
        .unwrap();
    metadata.increment_currently_processing_batch_index_if_full();
    assert_eq!(metadata.currently_processing_batch_index, 0);
}

#[test]
fn test_batch_size_validation() {
    // Test invalid batch size
    assert!(BatchMetadata::new_input_queue(10, 10, 3, 3, 0).is_err());
    assert!(BatchMetadata::new_output_queue(10, 3).is_err());

    // Test valid batch size
    assert!(BatchMetadata::new_input_queue(9, 10, 3, 3, 0).is_ok());
    assert!(BatchMetadata::new_output_queue(9, 3).is_ok());
}
