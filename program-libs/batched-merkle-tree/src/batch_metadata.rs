use light_merkle_tree_metadata::{errors::MerkleTreeMetadataError, queue::QueueType};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::{batch::BatchState, errors::BatchedMerkleTreeError, BorshDeserialize, BorshSerialize};

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
}

impl BatchMetadata {
    pub fn get_num_zkp_batches(&self) -> u64 {
        self.batch_size / self.zkp_batch_size
    }

    pub fn new_output_queue(
        batch_size: u64,
        zkp_batch_size: u64,
        num_batches: u64,
    ) -> Result<Self, BatchedMerkleTreeError> {
        if batch_size % zkp_batch_size != 0 {
            return Err(BatchedMerkleTreeError::BatchSizeNotDivisibleByZkpBatchSize);
        }
        Ok(BatchMetadata {
            num_batches,
            zkp_batch_size,
            batch_size,
            currently_processing_batch_index: 0,
            next_full_batch_index: 0,
            bloom_filter_capacity: 0,
        })
    }

    pub fn new_input_queue(
        batch_size: u64,
        bloom_filter_capacity: u64,
        zkp_batch_size: u64,
        num_batches: u64,
    ) -> Result<Self, BatchedMerkleTreeError> {
        if batch_size % zkp_batch_size != 0 {
            return Err(BatchedMerkleTreeError::BatchSizeNotDivisibleByZkpBatchSize);
        }
        Ok(BatchMetadata {
            num_batches,
            zkp_batch_size,
            batch_size,
            currently_processing_batch_index: 0,
            next_full_batch_index: 0,
            bloom_filter_capacity,
        })
    }

    /// Increment the next full batch index if current state is BatchState::Inserted.
    pub fn increment_next_full_batch_index_if_inserted(&mut self, state: BatchState) {
        if state == BatchState::Inserted {
            self.next_full_batch_index += 1;
            self.next_full_batch_index %= self.num_batches;
        }
    }

    /// Increment the currently_processing_batch_index if current state is BatchState::Full.
    pub fn increment_currently_processing_batch_index_if_full(&mut self, state: BatchState) {
        if state == BatchState::Full {
            self.currently_processing_batch_index += 1;
            self.currently_processing_batch_index %= self.num_batches;
        }
    }

    pub fn init(
        &mut self,
        num_batches: u64,
        batch_size: u64,
        zkp_batch_size: u64,
    ) -> Result<(), BatchedMerkleTreeError> {
        self.num_batches = num_batches;
        self.batch_size = batch_size;
        // Check that batch size is divisible by zkp_batch_size.
        if batch_size % zkp_batch_size != 0 {
            return Err(BatchedMerkleTreeError::BatchSizeNotDivisibleByZkpBatchSize);
        }
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
}

#[test]
fn test_increment_next_full_batch_index_if_inserted() {
    let mut metadata = BatchMetadata::new_input_queue(10, 10, 10, 2).unwrap();
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
    let mut metadata = BatchMetadata::new_input_queue(10, 10, 10, 2).unwrap();
    assert_eq!(metadata.currently_processing_batch_index, 0);
    // increment currently_processing_batch_index
    metadata.increment_currently_processing_batch_index_if_full(BatchState::Full);
    assert_eq!(metadata.currently_processing_batch_index, 1);
    // increment currently_processing_batch_index
    metadata.increment_currently_processing_batch_index_if_full(BatchState::Full);
    assert_eq!(metadata.currently_processing_batch_index, 0);
    // try incrementing next full batch index with state not full
    metadata.increment_currently_processing_batch_index_if_full(BatchState::Fill);
    assert_eq!(metadata.currently_processing_batch_index, 0);
    metadata.increment_currently_processing_batch_index_if_full(BatchState::Inserted);
    assert_eq!(metadata.currently_processing_batch_index, 0);
}
