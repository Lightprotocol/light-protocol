use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct Nullifier {
    pub nullifiers: Vec<NullifyEvent>,
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
#[repr(C)]
pub enum NullifyEvent {
    V1(NullifyEventV1),
}

/// Version 1 of the [`NullifyEvent`](account_compression::state::NullifyEvent).
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct NullifyEventV1 {
    /// Public key of the tree.
    pub id: [u8; 32],
    /// Leaf index.
    pub index: u64,
}
