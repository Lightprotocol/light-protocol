const MERKLE_TREE_ACC_BYTES_0: [u8; 32] = [
    242, 149, 147, 41, 62, 228, 214, 222, 231, 159, 167, 195, 10, 226, 182, 153, 84, 80, 249, 150,
    131, 112, 150, 225, 133, 131, 32, 149, 69, 188, 94, 13,
];

const MERKLE_TREE_TOKEN_ACC_BYTES_0: [u8; 32] = [
    123, 30, 128, 110, 93, 171, 2, 242, 20, 194, 175, 25, 246, 98, 182, 99, 31, 110, 119, 163, 68,
    179, 244, 89, 176, 19, 93, 136, 149, 231, 179, 213,
];

pub const MERKLE_TREE_ACC_BYTES_ARRAY: [([u8; 32], [u8; 32]); 1] =
    [(MERKLE_TREE_ACC_BYTES_0, MERKLE_TREE_TOKEN_ACC_BYTES_0)];

pub const MERKLE_TREE_INIT_AUTHORITY: [u8; 32] = [
    2, 99, 226, 251, 88, 66, 92, 33, 25, 216, 211, 185, 112, 203, 212, 238, 105, 144, 72, 121, 176,
    253, 106, 168, 115, 158, 154, 188, 62, 255, 166, 81,
];

pub const STORAGE_SEED: &[u8] = b"storage";
pub const ESCROW_SEED: &[u8] = b"fee_escrow";

pub const TIMEOUT_ESCROW: u64 = 300;

pub const FEE_PER_INSTRUCTION: u64 = 5000;

pub const VERIFIER_INDEX:u64 = 0;
