use crate::{BorshDeserialize, BorshSerialize};

#[repr(C)]
#[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq, Clone, Eq)]
pub struct BatchEvent {
    pub merkle_tree_pubkey: [u8; 32],
    pub batch_index: u64,
    pub zkp_batch_index: u64,
    pub zkp_batch_size: u64,
    pub old_next_index: u64,
    pub new_next_index: u64,
    pub new_root: [u8; 32],
    pub root_index: u32,
    pub sequence_number: u64,
    pub output_queue_pubkey: Option<[u8; 32]>,
}
