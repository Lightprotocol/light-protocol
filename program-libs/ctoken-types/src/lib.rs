pub mod instructions;

pub mod hash_cache;

pub mod error;

pub use error::*;
pub mod state;

// Re-export Pubkey type
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
use light_macros::pubkey_array;

pub const CPI_AUTHORITY: [u8; 32] = pubkey_array!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy");
pub const COMPRESSED_TOKEN_PROGRAM_ID: [u8; 32] =
    pubkey_array!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

/// Account size constants
/// Size of a basic SPL token account
pub const BASIC_TOKEN_ACCOUNT_SIZE: u64 = 165;

/// Size of a token account with compressible extension
pub const COMPRESSIBLE_TOKEN_ACCOUNT_SIZE: u64 = 257;
pub const COMPRESSED_MINT_SEED: &[u8] = b"compressed_mint";
pub const NATIVE_MINT: [u8; 32] = pubkey_array!("So11111111111111111111111111111111111111112");
