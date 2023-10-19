use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;

use crate::utils::config::MERKLE_TREE_HEIGHT;

#[account(zero_copy)]
#[aligned_sized(anchor)]
#[derive(Debug)]
#[repr(C)]
pub struct MerkleTreeUpdateState {
    pub node_left: [u8; 32],
    pub node_right: [u8; 32],
    pub leaf_left: [u8; 32],
    pub leaf_right: [u8; 32],
    pub relayer: Pubkey,
    pub merkle_tree_pda_pubkey: Pubkey,
    //
    pub state: [u8; 96],
    pub current_round: u64,
    pub current_round_index: u64,
    pub current_instruction_index: u64,
    pub current_index: u64,
    pub current_level: u64,
    pub current_level_hash: [u8; 32],
    pub tmp_leaves_index: u64,
    pub filled_subtrees: [[u8; 32]; MERKLE_TREE_HEIGHT],

    pub leaves: [[[u8; 32]; 2]; 16],
    pub number_of_leaves: u8,
    _padding1: [u8; 7],
    pub insert_leaves_index: u8,
    _padding2: [u8; 7],
}
