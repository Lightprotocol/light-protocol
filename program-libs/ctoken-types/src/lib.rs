pub mod instructions;

pub mod calculate_top_up;
pub mod error;
pub mod hash_cache;

pub use error::*;
pub mod state;
// TODO: cleanup this crate
// TODO: move all constants to constants.

#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
use light_macros::pubkey_array;

use crate::state::CompressionInfo;

pub const CPI_AUTHORITY: [u8; 32] = pubkey_array!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy");
pub const COMPRESSED_TOKEN_PROGRAM_ID: [u8; 32] =
    pubkey_array!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

/// Account size constants
/// Size of a basic SPL token account
pub const BASE_TOKEN_ACCOUNT_SIZE: u64 = 165;

/// Extension metadata overhead: AccountType (1) + Option discriminator (1) + Vec length (4) + Extension enum variant (1)
pub const EXTENSION_METADATA: u64 = 7;

/// Size of a token account with compressible extension
pub const COMPRESSIBLE_TOKEN_ACCOUNT_SIZE: u64 =
    BASE_TOKEN_ACCOUNT_SIZE + CompressionInfo::LEN as u64 + EXTENSION_METADATA;

/// Size of a Token-2022 mint account
pub const MINT_ACCOUNT_SIZE: u64 = 82;
pub const COMPRESSED_MINT_SEED: &[u8] = b"compressed_mint";
pub const NATIVE_MINT: [u8; 32] = pubkey_array!("So11111111111111111111111111111111111111112");

pub const CMINT_ADDRESS_TREE: [u8; 32] =
    pubkey_array!("EzKE84aVTkCUhDHLELqyJaq1Y7UVVmqxXqZjVHwHY3rK");
