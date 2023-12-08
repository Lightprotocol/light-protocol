use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use light_hasher::Poseidon;
use light_macros::pubkey;
use light_merkle_tree::{config::MerkleTreeConfig, MerkleTree};

use crate::impl_indexed_merkle_tree;

#[derive(Clone, Copy)]
pub struct TransactionMerkleTreeConfig {}

impl MerkleTreeConfig for TransactionMerkleTreeConfig {
    const PROGRAM_ID: Pubkey = pubkey!("JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6");
}

// NOTE(vadorovsky): This implementation of Merkle tree exists only for
// transactions and handling Poseidon in multiple rounds. Once Poseidon syscall
// in Solana is implemented, this implementation will be replaced with
// light-merkle-tree crate.
#[account(zero_copy)]
#[aligned_sized(anchor)]
pub struct TransactionMerkleTree {
    pub merkle_tree: MerkleTree<Poseidon, TransactionMerkleTreeConfig>,
    pub pubkey_locked: Pubkey,
    pub time_locked: u64,
    pub merkle_tree_nr: u64,
    pub lock_duration: u64,
    pub next_queued_index: u64,
    pub newest: u8,
    _padding: [u8; 7],
}

impl_indexed_merkle_tree!(TransactionMerkleTree);

#[account]
#[aligned_sized(anchor)]
#[derive(Eq, PartialEq, Debug)]
pub struct TwoLeavesBytesPda {
    pub node_left: [u8; 32],
    pub node_right: [u8; 32],
    pub merkle_tree_pubkey: Pubkey,
    pub encrypted_utxos: [u8; 256],
    pub left_leaf_index: u64,
}
