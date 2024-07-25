// This file stores constants which do not have to be configured.
use anchor_lang::constant;

#[constant]
pub const CPI_AUTHORITY_PDA_SEED: &[u8] = b"cpi_authority";

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
pub const STATE_NULLIFIER_QUEUE_VALUES: u16 = 28_807;
#[constant]
pub const STATE_NULLIFIER_QUEUE_SEQUENCE_THRESHOLD: u64 = 2400;

#[constant]
pub const ADDRESS_MERKLE_TREE_HEIGHT: u64 = 26;
#[constant]
pub const ADDRESS_MERKLE_TREE_CHANGELOG: u64 = 1400;
#[constant]
pub const ADDRESS_MERKLE_TREE_ROOTS: u64 = 2400;
#[constant]
pub const ADDRESS_MERKLE_TREE_CANOPY_DEPTH: u64 = 10;
#[constant]
pub const ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG: u64 = 1400;

#[constant]
pub const ADDRESS_QUEUE_VALUES: u16 = 28_807;
#[constant]
pub const ADDRESS_QUEUE_SEQUENCE_THRESHOLD: u64 = 2400;
// noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV
#[constant]
pub const NOOP_PUBKEY: [u8; 32] = [
    11, 188, 15, 192, 187, 71, 202, 47, 116, 196, 17, 46, 148, 171, 19, 207, 163, 198, 52, 229,
    220, 23, 234, 203, 3, 205, 26, 35, 205, 126, 120, 124,
];
