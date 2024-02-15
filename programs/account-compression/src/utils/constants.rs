// This file stores constants which do not have to be configured.
use anchor_lang::constant;

#[constant]
pub const GROUP_AUTHORITY_SEED: &[u8] = b"group_authority";

#[constant]
pub const MERKLE_TREE_HEIGHT: usize = 22;
#[constant]
pub const MERKLE_TREE_CHANGELOG: usize = 0;
#[constant]
pub const MERKLE_TREE_ROOTS: usize = 2800;
