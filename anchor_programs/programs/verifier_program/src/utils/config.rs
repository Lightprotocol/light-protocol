/*const MERKLE_TREE_ACC_BYTES_0: [u8; 32] = [
  190, 128,   2, 139, 132, 166, 200,
  112, 236,  75,  16,  77, 200, 175,
  154, 124, 163, 241, 240, 136,  11,
   14, 233, 211,  37, 101, 200, 190,
  101, 163, 127,  20
];

const MERKLE_TREE_TOKEN_ACC_BYTES_0: [u8; 32] = [
  218, 24,  22, 174,  97, 242, 114,  92,
   10, 17, 126,  18, 203, 163, 145, 123,
    3, 83, 209, 157, 145, 202, 112, 112,
  133, 88,   2, 242, 144,  12, 225,  72
];

pub const MERKLE_TREE_ACC_BYTES_ARRAY: [([u8; 32], [u8; 32]); 1] =
    [(MERKLE_TREE_ACC_BYTES_0, MERKLE_TREE_TOKEN_ACC_BYTES_0)];

pub const MERKLE_TREE_INIT_AUTHORITY: [u8; 32] = [
    2, 99, 226, 251, 88, 66, 92, 33, 25, 216, 211, 185, 112, 203, 212, 238, 105, 144, 72, 121, 176,
    253, 106, 168, 115, 158, 154, 188, 62, 255, 166, 81,
];
*/
pub const STORAGE_SEED: &[u8] = b"storage";
pub const ESCROW_SEED: &[u8] = b"escrow";

pub const TIMEOUT_ESCROW: u64 = 300;

pub const FEE_PER_INSTRUCTION: u64 = 5000;

pub const VERIFIER_INDEX:u64 = 0;
