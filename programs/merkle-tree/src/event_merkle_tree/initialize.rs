use std::cell::RefMut;

use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use light_hasher::Sha256;
use light_macros::pubkey;
use light_merkle_tree::{config::MerkleTreeConfig, HashFunction, MerkleTree};

use crate::{impl_indexed_merkle_tree, utils::config::MERKLE_TREE_HEIGHT, MerkleTreeAuthority};

#[derive(Clone, Copy)]
pub struct EventMerkleTreeConfig {}

impl MerkleTreeConfig for EventMerkleTreeConfig {
    const PROGRAM_ID: Pubkey = pubkey!("JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6");
}

#[account(zero_copy)]
#[aligned_sized(anchor)]
pub struct EventMerkleTree {
    pub merkle_tree_nr: u64,
    pub newest: u8,
    _padding: [u8; 7],
    pub merkle_tree: MerkleTree<Sha256, EventMerkleTreeConfig>,
}

impl_indexed_merkle_tree!(EventMerkleTree);

pub fn process_initialize_new_event_merkle_tree(
    merkle_tree: &mut RefMut<'_, EventMerkleTree>,
    merkle_tree_authority: &mut Account<'_, MerkleTreeAuthority>,
) -> Result<()> {
    merkle_tree
        .merkle_tree
        .init(MERKLE_TREE_HEIGHT, HashFunction::Sha256)?;
    merkle_tree.merkle_tree_nr = merkle_tree_authority.event_merkle_tree_index;
    merkle_tree.newest = 1;

    merkle_tree_authority.event_merkle_tree_index += 1;

    Ok(())
}
