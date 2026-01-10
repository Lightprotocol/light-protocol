use light_macros::pubkey_array;

pub const CPI_AUTHORITY: [u8; 32] = pubkey_array!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy");
pub const CTOKEN_PROGRAM_ID: [u8; 32] =
    pubkey_array!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

/// Account size constants
/// Size of a CToken account with embedded compression info (no extensions).
/// CTokenZeroCopy includes: SPL token layout (165) + account_type (1) + decimal_option_prefix (1)
/// + decimals (1) + compression_only (1) + CompressionInfo (96) + has_extensions (1)
pub use crate::state::BASE_TOKEN_ACCOUNT_SIZE;

/// Extension metadata overhead: Vec length (4) - added when any extensions are present
/// Note: The Option discriminator is the has_extensions bool in the base struct
pub const EXTENSION_METADATA: u64 = 4;

/// Size of CompressedOnly extension (16 bytes for two u64 fields: delegated_amount and withheld_transfer_fee)
pub const COMPRESSED_ONLY_EXTENSION_SIZE: u64 = 16;

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

/// Pool PDA seeds
pub const POOL_SEED: &[u8] = b"pool";
pub const RESTRICTED_POOL_SEED: &[u8] = b"restricted";
