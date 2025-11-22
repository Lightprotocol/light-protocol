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

/// Size of a token account with compressible extension 261 bytes.
/// CompressibleExtension: 1 byte compression_only + 88 bytes CompressionInfo
pub const COMPRESSIBLE_TOKEN_ACCOUNT_SIZE: u64 =
    BASE_TOKEN_ACCOUNT_SIZE + 1 + CompressionInfo::LEN as u64 + EXTENSION_METADATA;

/// Size of a token account with compressible + pausable extensions (262 bytes).
/// Adds 1 byte for PausableAccount discriminator (marker extension with 0 data bytes).
pub const COMPRESSIBLE_PAUSABLE_TOKEN_ACCOUNT_SIZE: u64 = COMPRESSIBLE_TOKEN_ACCOUNT_SIZE + 1;

/// Rent exemption threshold for compressible token accounts (in lamports)
/// This value determines when an account has sufficient rent to be considered not compressible
pub const COMPRESSIBLE_TOKEN_RENT_EXEMPTION: u64 = 2700480;

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

/// Calculates the size of a ctoken account based on which extensions are present.
///
/// # Arguments
/// * `has_compressible` - Whether the account has the Compressible extension
/// * `has_pausable` - Whether the account has the PausableAccount extension (marker, 0 bytes)
/// * `has_permanent_delegate` - Whether the account has the PermanentDelegateAccount extension (marker, 0 bytes)
/// * `has_transfer_fee` - Whether the account has the TransferFeeAccount extension (8 bytes)
/// * `has_transfer_hook` - Whether the account has the TransferHookAccount extension (1 byte transferring)
///
/// # Returns
/// The total account size in bytes
///
/// # Extension Sizes
/// - Base account: 165 bytes
/// - Extension metadata (per extension): 7 bytes (1 AccountType + 1 Option + 4 Vec len + 1 discriminant)
/// - Compressible: 89 bytes (1 compression_only + 88 CompressionInfo::LEN)
/// - PausableAccount: 0 bytes (marker only, just discriminant)
/// - PermanentDelegateAccount: 0 bytes (marker only, just discriminant)
/// - TransferFeeAccount: 8 bytes (withheld_amount u64)
/// - TransferHookAccount: 1 byte (transferring flag, consistent with T22)
pub const fn calculate_ctoken_account_size(
    has_compressible: bool,
    has_pausable: bool,
    has_permanent_delegate: bool,
    has_transfer_fee: bool,
    has_transfer_hook: bool,
) -> u64 {
    let mut size = BASE_TOKEN_ACCOUNT_SIZE;

    if has_compressible {
        // CompressibleExtension: 1 byte compression_only + CompressionInfo::LEN
        size += 1 + CompressionInfo::LEN as u64 + EXTENSION_METADATA;
    }

    if has_pausable {
        // PausableAccount is a marker extension (0 data bytes), just adds discriminant
        size += 1;
    }

    if has_permanent_delegate {
        // PermanentDelegateAccount is a marker extension (0 data bytes), just adds discriminant
        size += 1;
    }

    if has_transfer_fee {
        // TransferFeeAccount: 1 discriminant + 8 withheld_amount
        size += TRANSFER_FEE_ACCOUNT_EXTENSION_LEN;
    }

    if has_transfer_hook {
        // TransferHookAccount: 1 discriminant + 1 transferring flag (consistent with T22)
        size += TRANSFER_HOOK_ACCOUNT_EXTENSION_LEN;
    }

    size
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ctoken_account_size_calculation() {
        // Base only (no extensions)
        assert_eq!(
            calculate_ctoken_account_size(false, false, false, false, false),
            BASE_TOKEN_ACCOUNT_SIZE
        );

        // With compressible only
        assert_eq!(
            calculate_ctoken_account_size(true, false, false, false, false),
            COMPRESSIBLE_TOKEN_ACCOUNT_SIZE
        );

        // With compressible + pausable
        assert_eq!(
            calculate_ctoken_account_size(true, true, false, false, false),
            COMPRESSIBLE_PAUSABLE_TOKEN_ACCOUNT_SIZE
        );

        // With compressible + pausable + permanent_delegate (262 + 1 = 263)
        assert_eq!(
            calculate_ctoken_account_size(true, true, true, false, false),
            263
        );

        // With pausable only (165 + 1 = 166)
        assert_eq!(
            calculate_ctoken_account_size(false, true, false, false, false),
            166
        );

        // With permanent_delegate only (165 + 1 = 166)
        assert_eq!(
            calculate_ctoken_account_size(false, false, true, false, false),
            166
        );

        // With pausable + permanent_delegate (165 + 1 + 1 = 167)
        assert_eq!(
            calculate_ctoken_account_size(false, true, true, false, false),
            167
        );

        // With compressible + permanent_delegate (261 + 1 = 262)
        assert_eq!(
            calculate_ctoken_account_size(true, false, true, false, false),
            262
        );

        // With transfer_fee only (165 + 9 = 174)
        assert_eq!(
            calculate_ctoken_account_size(false, false, false, true, false),
            174
        );

        // With compressible + transfer_fee (261 + 9 = 270)
        assert_eq!(
            calculate_ctoken_account_size(true, false, false, true, false),
            270
        );

        // With 4 extensions (261 + 1 + 1 + 9 = 272)
        assert_eq!(
            calculate_ctoken_account_size(true, true, true, true, false),
            272
        );

        // With all 5 extensions (261 + 1 + 1 + 9 + 2 = 274)
        assert_eq!(
            calculate_ctoken_account_size(true, true, true, true, true),
            274
        );

        // With transfer_hook only (165 + 2 = 167)
        assert_eq!(
            calculate_ctoken_account_size(false, false, false, false, true),
            167
        );
    }
}
