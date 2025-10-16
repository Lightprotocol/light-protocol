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

/// Size of a token account with compressible extension 260 bytes.
pub const COMPRESSIBLE_TOKEN_ACCOUNT_SIZE: u64 =
    BASE_TOKEN_ACCOUNT_SIZE + CompressionInfo::LEN as u64 + EXTENSION_METADATA;

/// Rent exemption threshold for compressible token accounts (in lamports)
/// This value determines when an account has sufficient rent to be considered not compressible
pub const COMPRESSIBLE_TOKEN_RENT_EXEMPTION: u64 = 2700480;

/// Size of a Token-2022 mint account
pub const MINT_ACCOUNT_SIZE: u64 = 82;
pub const COMPRESSED_MINT_SEED: &[u8] = b"compressed_mint";
pub const NATIVE_MINT: [u8; 32] = pubkey_array!("So11111111111111111111111111111111111111112");

pub const CMINT_ADDRESS_TREE: [u8; 32] =
    pubkey_array!("amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx");
