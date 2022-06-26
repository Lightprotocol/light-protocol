use crate::ENCRYPTED_UTXOS_LENGTH;
// This file stores constants which do not have to be configured.


pub const ROOT_CHECK: u8 = 15;
pub const TWO_LEAVES_PDA_SIZE: u64 = 106 + ENCRYPTED_UTXOS_LENGTH as u64;
//instruction order
pub const IX_ORDER: [u8; 57] = [
    34, 14, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2,
    0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2,
    0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 241
];
// Identitifiers for instructions
pub const MERKLE_TREE_UPDATE_START: u8 = 14;
pub const MERKLE_TREE_UPDATE_LEVEL: u8 = 25;
pub const LOCK_START: u8 = 34;
pub const HASH_0: u8 = 0;
pub const HASH_1: u8 = 1;
pub const HASH_2: u8 = 2;
pub const ROOT_INSERT: u8 = 241;
