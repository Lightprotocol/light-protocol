use crate::{pubkey, Pubkey};

/// Seed of the CPI authority.
pub const CPI_AUTHORITY_PDA_SEED: &[u8] = b"cpi_authority";

/// ID of the account-compression program.
pub const PROGRAM_ID_ACCOUNT_COMPRESSION: Pubkey =
    pubkey!("compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq");
pub const PROGRAM_ID_NOOP: Pubkey = pubkey!("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");
/// ID of the light-system program.
pub const PROGRAM_ID_LIGHT_SYSTEM: Pubkey = pubkey!("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7");
/// ID of the light-compressed-token program.
pub const PROGRAM_ID_LIGHT_TOKEN: Pubkey = pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

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
