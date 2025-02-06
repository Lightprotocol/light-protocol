use crate::{BorshDeserialize, BorshSerialize};

#[repr(C)]
#[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq)]
pub struct BatchAppendEvent {
    // TODO: rename to merkle tree pubkey
    pub id: [u8; 32],
    // TODO: add tree type
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
}

#[repr(C)]
#[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq)]
pub struct BatchNullifyEvent {
    // TODO: rename to merkle tree pubkey
    pub id: [u8; 32],
    pub batch_index: u64,
    pub zkp_batch_index: u64,
    /// Zkp batch size.
    pub batch_size: u64,
    pub new_root: [u8; 32],
    pub root_index: u32,
    pub sequence_number: u64,
}

// TODO: use append event so that it contains next index for address event
pub type BatchAddressAppendEvent = BatchNullifyEvent;
