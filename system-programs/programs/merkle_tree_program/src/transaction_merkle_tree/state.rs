use crate::{
    impl_indexed_merkle_tree,
    utils::config::{MERKLE_TREE_HEIGHT, MERKLE_TREE_HISTORY_SIZE},
};
use anchor_lang::prelude::*;

// NOTE(vadorovsky): This implementation of Merkle tree exists only for
// transactions and handling Poseidon in multiple rounds. Once Poseidon syscall
// in Solana is implemented, this implementation will be replaced with
// light-merkle-tree crate.
#[account(zero_copy)]
#[derive(Eq, PartialEq, Debug)]
pub struct TransactionMerkleTree {
    pub filled_subtrees: [[u8; 32]; MERKLE_TREE_HEIGHT],
    pub current_root_index: u64,
    pub next_index: u64,
    pub roots: [[u8; 32]; MERKLE_TREE_HISTORY_SIZE as usize],
    pub pubkey_locked: Pubkey,
    pub time_locked: u64,
    pub height: u64,
    pub merkle_tree_nr: u64,
    pub lock_duration: u64,
    pub next_queued_index: u64,
    pub newest: u8,
    _padding: [u8; 7],
}

impl_indexed_merkle_tree!(TransactionMerkleTree);

#[account]
#[derive(Eq, PartialEq, Debug)]
pub struct TwoLeavesBytesPda {
    pub node_left: [u8; 32],
    pub node_right: [u8; 32],
    pub merkle_tree_pubkey: Pubkey,
    pub encrypted_utxos: [u8; 256],
    pub left_leaf_index: u64,
}
