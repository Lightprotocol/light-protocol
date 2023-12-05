pub mod poseidon;
pub mod sha256;

pub const MAX_HEIGHT: usize = 32;
pub const MAX_ROOTS: usize = 256;
pub const DATA_LEN: usize = 32;
pub const HASH_LEN: usize = 32;

pub type ZeroBytes = [[u8; 32]; MAX_HEIGHT + 1];
