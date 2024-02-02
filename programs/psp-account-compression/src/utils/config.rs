use anchor_lang::constant;

#[constant]
pub const ENCRYPTED_UTXOS_LENGTH: usize = 174;

#[constant]
pub const MERKLE_TREE_HEIGHT: usize = 22;
#[constant]
pub const MERKLE_TREE_CHANGELOG: usize = 0;
#[constant]
pub const MERKLE_TREE_ROOTS: usize = 2800;

#[constant]
pub const INITIAL_MERKLE_TREE_AUTHORITY: [u8; 32] = [
    2, 99, 226, 251, 88, 66, 92, 33, 25, 216, 211, 185, 112, 203, 212, 238, 105, 144, 72, 121, 176,
    253, 106, 168, 115, 158, 154, 188, 62, 255, 166, 81,
];
