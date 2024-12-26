use bytemuck::{Pod, Zeroable};

use crate::{BorshDeserialize, BorshSerialize};

#[repr(C)]
#[derive(
    BorshDeserialize, BorshSerialize, Debug, PartialEq, Default, Pod, Zeroable, Clone, Copy,
)]
pub struct BatchMetadata {
    pub num_batches: u64,
    pub batch_size: u64,
    pub zkp_batch_size: u64,
    pub currently_processing_batch_index: u64,
    pub next_full_batch_index: u64,
    pub bloom_filter_capacity: u64,
}

impl BatchMetadata {
    pub fn get_num_zkp_batches(&self) -> u64 {
        self.batch_size / self.zkp_batch_size
    }

    pub fn new_output_queue(batch_size: u64, zkp_batch_size: u64, num_batches: u64) -> Self {
        BatchMetadata {
            num_batches,
            zkp_batch_size,
            batch_size,
            currently_processing_batch_index: 0,
            next_full_batch_index: 0,
            bloom_filter_capacity: 0,
        }
    }

    pub fn new_input_queue(
        batch_size: u64,
        bloom_filter_capacity: u64,
        zkp_batch_size: u64,
        num_batches: u64,
    ) -> Self {
        BatchMetadata {
            num_batches,
            zkp_batch_size,
            batch_size,
            currently_processing_batch_index: 0,
            next_full_batch_index: 0,
            bloom_filter_capacity,
        }
    }
}
