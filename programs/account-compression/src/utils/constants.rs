// This file stores constants which do not have to be configured.
use anchor_lang::constant;

#[constant]
pub const GROUP_AUTHORITY_SEED: &[u8] = b"group_authority";

#[constant]
pub const STATE_MERKLE_TREE_HEIGHT: usize = 26;
#[constant]
pub const STATE_MERKLE_TREE_CHANGELOG: usize = 1400;
#[constant]
pub const STATE_MERKLE_TREE_ROOTS: usize = 2400;
#[constant]
pub const STATE_MERKLE_TREE_CANOPY_DEPTH: usize = 10;

#[constant]
pub const STATE_INDEXED_ARRAY_SIZE: usize = 4800;

#[constant]
pub const ADDRESS_MERKLE_TREE_HEIGHT: usize = 22;
#[constant]
pub const ADDRESS_MERKLE_TREE_CHANGELOG: usize = 2800;
#[constant]
pub const ADDRESS_MERKLE_TREE_ROOTS: usize = 2800;
#[constant]
pub const ADDRESS_MERKLE_TREE_CANOPY_DEPTH: usize = 0;
