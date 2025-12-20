use light_macros::pubkey_array;

use crate::state::extensions::CompressibleExtension;

pub const CPI_AUTHORITY: [u8; 32] = pubkey_array!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy");
pub const CTOKEN_PROGRAM_ID: [u8; 32] =
    pubkey_array!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

/// Account size constants
/// Size of a basic SPL token account
pub const BASE_TOKEN_ACCOUNT_SIZE: u64 = 165;

/// Extension metadata overhead: AccountType (1) + Option discriminator (1) + Vec length (4) + Extension enum variant (1)
pub const EXTENSION_METADATA: u64 = 7;

/// Size of a token account with compressible extension (263 bytes).
/// CompressibleExtension: 1 compression_only + 1 decimals + 1 has_decimals + 88 CompressionInfo
pub const COMPRESSIBLE_TOKEN_ACCOUNT_SIZE: u64 =
    BASE_TOKEN_ACCOUNT_SIZE + CompressibleExtension::LEN as u64 + EXTENSION_METADATA;

/// Size of a token account with compressible + pausable extensions (264 bytes).
/// Adds 1 byte for PausableAccount discriminator (marker extension with 0 data bytes).
pub const COMPRESSIBLE_PAUSABLE_TOKEN_ACCOUNT_SIZE: u64 = COMPRESSIBLE_TOKEN_ACCOUNT_SIZE + 1;

/// Size of CompressedOnly extension (8 bytes for u64 delegated_amount)
pub const COMPRESSED_ONLY_EXTENSION_SIZE: u64 = 8;

/// Size of a Token-2022 mint account
pub const MINT_ACCOUNT_SIZE: u64 = 82;
pub const COMPRESSED_MINT_SEED: &[u8] = b"compressed_mint";
pub const NATIVE_MINT: [u8; 32] = pubkey_array!("So11111111111111111111111111111111111111112");

pub const CMINT_ADDRESS_TREE: [u8; 32] =
    pubkey_array!("amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx");

/// Size of TransferFeeAccountExtension: 1 discriminant + 8 withheld_amount
pub const TRANSFER_FEE_ACCOUNT_EXTENSION_LEN: u64 = 9;

/// Size of TransferHookAccountExtension: 1 discriminant + 1 transferring
pub const TRANSFER_HOOK_ACCOUNT_EXTENSION_LEN: u64 = 2;

/// Instruction discriminator for Transfer2
pub const TRANSFER2: u8 = 101;
