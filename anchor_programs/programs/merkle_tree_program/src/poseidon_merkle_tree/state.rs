use crate::utils::config::{
    ENCRYPTED_UTXOS_LENGTH,
    MERKLE_TREE_HISTORY_SIZE
};
use crate::utils::constants::MERKLE_TREE_ACCOUNT_TYPE;
use anchor_lang::prelude::*;


#[account(zero_copy)]
#[derive(Eq, PartialEq, Debug)]
pub struct MerkleTree {
    pub filled_subtrees: [[u8;32];18],
    pub current_root_index: u64,
    pub next_index: u64,
    pub roots: [[u8;32];MERKLE_TREE_HISTORY_SIZE as usize],
    pub pubkey_locked: Pubkey,
    pub time_locked: u64,
    pub height: u64,
    pub merkle_tree_nr: u64,
}

#[account]
#[derive(Eq, PartialEq, Debug)]
pub struct TwoLeavesBytesPda {
    pub node_left: [u8;32],
    pub node_right: [u8;32],
    pub merkle_tree_pubkey: Pubkey,
    pub encrypted_utxos: [u8; 256],
    pub left_leaf_index: u64
}
