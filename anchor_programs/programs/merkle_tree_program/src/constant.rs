use crate::ENCRYPTED_UTXOS_LENGTH;

pub const ROOT_CHECK: u8 = 15;
pub const TWO_LEAVES_PDA_SIZE: u64 = 106 + ENCRYPTED_UTXOS_LENGTH as u64;
//instruction order
// pub const IX_ORDER: [u8; 74] = [
//     34, 14, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2,
//     25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25,
//     0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2,  //perform last checks and transfer requested amount
//     241,
// ];
pub const IX_ORDER: [u8; 57] = [
    34, 14, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2,
    0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2,
    0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 241
];
pub const MERKLE_TREE_UPDATE_START: u8 = 14;
pub const MERKLE_TREE_UPDATE_LEVEL: u8 = 25;

pub const LOCK_START: u8 = 34;

// duration measured in slots
pub const LOCK_DURATION: u64 = 600;
pub const HASH_0: u8 = 0;
pub const HASH_1: u8 = 1;
pub const HASH_2: u8 = 2;
pub const ROOT_INSERT: u8 = 241;
