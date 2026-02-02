// Re-export core constants from light-compressed-account
pub use light_compressed_account::constants::{
    ACCOUNT_COMPRESSION_AUTHORITY_PDA, ACCOUNT_COMPRESSION_PROGRAM_ID, CPI_AUTHORITY_PDA_SEED,
    LIGHT_SYSTEM_PROGRAM_ID, REGISTERED_PROGRAM_PDA,
};
use light_macros::pubkey_array;

/// ID of the light-compressed-token program.
pub const LIGHT_TOKEN_PROGRAM_ID: [u8; 32] =
    pubkey_array!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");
/// Seed of the rent sponsor PDA.
pub const RENT_SPONSOR_SEED: &[u8] = b"rent_sponsor";
pub const NOOP_PROGRAM_ID: [u8; 32] = pubkey_array!("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");

pub const STATE_MERKLE_TREE_HEIGHT: usize = 26;
pub const STATE_MERKLE_TREE_CHANGELOG: usize = 1400;
pub const STATE_MERKLE_TREE_ROOTS: usize = 2400;
pub const STATE_MERKLE_TREE_CANOPY_DEPTH: usize = 10;

pub const ADDRESS_MERKLE_TREE_HEIGHT: usize = 26;
pub const ADDRESS_MERKLE_TREE_CHANGELOG: usize = 1400;
pub const ADDRESS_MERKLE_TREE_ROOTS: usize = 2400;
pub const ADDRESS_MERKLE_TREE_CANOPY_DEPTH: usize = 10;
pub const ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG: usize = 1400;

pub const TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR: [u8; 8] = [2, 0, 0, 0, 0, 0, 0, 0];

pub const ADDRESS_TREE_V1: [u8; 32] = pubkey_array!("amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2");
pub const ADDRESS_QUEUE_V1: [u8; 32] = pubkey_array!("aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F");
pub const CPI_CONTEXT_ACCOUNT_1_DISCRIMINATOR: [u8; 8] = [22, 20, 149, 218, 74, 204, 128, 166];
pub const CPI_CONTEXT_ACCOUNT_2_DISCRIMINATOR: [u8; 8] = [34, 184, 183, 14, 100, 80, 183, 124];

pub const SOL_POOL_PDA: [u8; 32] = pubkey_array!("CHK57ywWSDncAoRu1F8QgwYJeXuAJyyBYT4LixLXvMZ1");

pub const ADDRESS_TREE_V2: [u8; 32] = pubkey_array!("amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx");

/// Default compressible config PDA for the Light Token Program (V1).
pub const LIGHT_TOKEN_CONFIG: [u8; 32] =
    pubkey_array!("ACXg8a7VaqecBWrSbdu73W4Pg9gsqXJ3EXAqkHyhvVXg");

/// Default rent sponsor PDA for the Light Token Program (V1).
pub const LIGHT_TOKEN_RENT_SPONSOR: [u8; 32] =
    pubkey_array!("r18WwUxfG8kQ69bQPAB2jV6zGNKy3GosFGctjQoV4ti");
