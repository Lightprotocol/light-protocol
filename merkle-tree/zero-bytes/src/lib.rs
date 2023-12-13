pub mod poseidon;
pub mod sha256;

pub const MAX_HEIGHT: usize = 64;

pub type ZeroBytes = [[u8; 32]; MAX_HEIGHT + 1];
