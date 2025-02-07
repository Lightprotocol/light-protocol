use crate::{BorshDeserialize, BorshSerialize};

pub const BATCH_EVENT_DISCRIMINATOR: [u8; 8] = *b"BatchEvt";

#[repr(C)]
#[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq)]
pub struct BatchAppendEvent {
    pub discriminator: [u8; 8],
    pub tree_type: u64,
    pub merkle_tree_pubkey: [u8; 32],
    pub batch_index: u64,
    pub zkp_batch_index: u64,
    /// Zkp batch size.
    pub batch_size: u64,
    /// Next leaf index. (Right most index of an append only Merkle tree.)
    pub old_next_index: u64,
    pub new_next_index: u64,
    pub new_root: [u8; 32],
    pub root_index: u32,
    pub sequence_number: u64,
    pub output_queue_pubkey: Option<[u8; 32]>,
}

pub type BatchNullifyEvent = BatchAppendEvent;

pub type BatchAddressAppendEvent = BatchAppendEvent;
