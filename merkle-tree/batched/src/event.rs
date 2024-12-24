use crate::{BorshDeserialize, BorshSerialize};

#[repr(C)]
#[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq)]
pub struct BatchAppendEvent {
    pub id: [u8; 32],
    pub batch_index: u64,
    pub zkp_batch_index: u64,
    pub batch_size: u64,
    pub old_next_index: u64,
    pub new_next_index: u64,
    pub new_root: [u8; 32],
    pub root_index: u32,
    pub sequence_number: u64,
}

#[repr(C)]
#[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq)]
pub struct BatchNullifyEvent {
    pub id: [u8; 32],
    pub batch_index: u64,
    pub zkp_batch_index: u64,
    pub new_root: [u8; 32],
    pub root_index: u32,
    pub sequence_number: u64,
    pub batch_size: u64,
}
