// This file stores constants which do not have to be configured.
use anchor_lang::constant;

#[constant]
pub const GROUP_AUTHORITY_SEED: &[u8] = b"group_authority";

#[constant]
pub const STATE_MERKLE_TREE_HEIGHT: u64 = 26;
#[constant]
pub const STATE_MERKLE_TREE_CHANGELOG: u64 = 1400;
#[constant]
pub const STATE_MERKLE_TREE_ROOTS: u64 = 2400;
#[constant]
pub const STATE_MERKLE_TREE_CANOPY_DEPTH: u64 = 10;

#[constant]
pub const STATE_INDEXED_ARRAY_INDICES: u16 = 6857;
#[constant]
pub const STATE_INDEXED_ARRAY_VALUES: u16 = 4800;
#[constant]
pub const STATE_INDEXED_ARRAY_SEQUENCE_THRESHOLD: u64 = 2400;

#[constant]
pub const ADDRESS_MERKLE_TREE_HEIGHT: u64 = 26;
#[constant]
pub const ADDRESS_MERKLE_TREE_CHANGELOG: u64 = 1400;
#[constant]
pub const ADDRESS_MERKLE_TREE_ROOTS: u64 = 2400;
#[constant]
pub const ADDRESS_MERKLE_TREE_CANOPY_DEPTH: u64 = 10;

#[constant]
pub const ADDRESS_QUEUE_INDICES: u16 = 6857;
#[constant]
pub const ADDRESS_QUEUE_VALUES: u16 = 4800;
#[constant]
pub const ADDRESS_QUEUE_SEQUENCE_THRESHOLD: u64 = 2400;
