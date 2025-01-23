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
    /// Bloom filter capacity in bits.
    pub bloom_filter_capacity: u64,
    /// Batch elements are currently inserted in.
    pub currently_processing_batch_index: u64,
    /// Next batch to be inserted into the tree.
    pub next_full_batch_index: u64,
    /// Output queues require next index to derive compressed account hashes.
    /// Output & Address queues append state hence need to check tree capacity.
    /// next_index in queue is ahead or equal to next index in the associated
    /// batched Merkle tree account.
    pub next_index: u64,
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

    /// Returns the size of the bloom filter in bytes.
    pub fn get_bloomfilter_size_bytes(&self) -> usize {
        (self.bloom_filter_capacity / 8) as usize
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
            next_index: 0,
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
            next_index: 0,
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
            solana_program::msg!("Incrementing next full batch index");
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
        let num_stores = if queue_type == QueueType::BatchedOutput as u64 {
            0
        } else {
            num_batches
        };
        Ok((num_value_stores, num_stores, num_batches))
    }

    pub fn queue_account_size(&self, queue_type: u64) -> Result<usize, BatchedMerkleTreeError> {
        let (num_value_vec, num_bloom_filter_stores, num_hashchain_store) =
            self.get_size_parameters(queue_type)?;
        let account_size = if queue_type == QueueType::BatchedInput as u64 {
            // Input queue is part of the Merkle tree account.
            0
        } else {
            // Output queue is a separate account.
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
        .advance_state_to_fill(None)
        .unwrap();
    metadata.increment_currently_processing_batch_index_if_full();
    assert_eq!(metadata.currently_processing_batch_index, 0);
}

#[test]
fn test_validate_batch_sizes() {
    assert!(BatchMetadata::validate_batch_sizes(10, 3).is_err());
    assert!(BatchMetadata::validate_batch_sizes(10, 2).is_ok());
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

#[test]
fn test_output_queue_account_size() {
    let metadata = BatchMetadata::new_output_queue(10, 2).unwrap();
    let queue_size = 472 + (16 + 10 * 32) * 2 + (16 + 5 * 32) * 2;
    assert_eq!(
        metadata
            .queue_account_size(QueueType::BatchedOutput as u64)
            .unwrap(),
        queue_size
    );
}

#[test]
fn test_imput_queue_account_size() {
    let metadata = BatchMetadata::new_input_queue(10, 20000 * 8, 2, 3, 0).unwrap();
    let queue_size = 20000 * 2 + (16 + 5 * 32) * 2;
    assert_eq!(
        metadata
            .queue_account_size(QueueType::BatchedInput as u64)
            .unwrap(),
        queue_size
    );
    assert_eq!(
        metadata.queue_account_size(4).unwrap_err(),
        MerkleTreeMetadataError::InvalidQueueType.into()
    );
}

#[test]
fn test_get_size_parameters() {
    let metadata = BatchMetadata::new_input_queue(10, 10, 2, 1, 0).unwrap();
    assert_eq!(
        metadata
            .get_size_parameters(QueueType::BatchedInput as u64)
            .unwrap(),
        (0, 2, 2)
    );
    assert_eq!(
        metadata
            .get_size_parameters(QueueType::BatchedOutput as u64)
            .unwrap(),
        (2, 0, 2)
    );
    assert_eq!(
        metadata
            .get_size_parameters(QueueType::NullifierQueue as u64)
            .unwrap_err(),
        MerkleTreeMetadataError::InvalidQueueType
    );
}

#[test]
fn test_init() {
    let mut metadata = BatchMetadata::new_output_queue(10, 2).unwrap();
    assert!(metadata.init(12, 5).is_err());
    assert!(metadata.init(10, 2).is_ok());
    assert_eq!(metadata.batch_size, 10);
    assert_eq!(metadata.zkp_batch_size, 2);
}

#[test]
fn test_get_num_zkp_batches() {
    let metadata = BatchMetadata::new_output_queue(10, 2).unwrap();
    assert_eq!(metadata.get_num_zkp_batches(), 5);
}

#[test]
fn test_get_current_batch() {
    let mut metadata = BatchMetadata::new_output_queue(10, 2).unwrap();
    assert_eq!(metadata.get_current_batch().get_state(), BatchState::Fill);
    metadata
        .get_current_batch_mut()
        .advance_state_to_full()
        .unwrap();
    assert_eq!(metadata.get_current_batch().get_state(), BatchState::Full);
    metadata
        .get_current_batch_mut()
        .advance_state_to_inserted()
        .unwrap();
    assert_eq!(
        metadata.get_current_batch().get_state(),
        BatchState::Inserted
    );
}
