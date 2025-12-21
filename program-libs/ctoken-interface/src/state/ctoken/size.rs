use light_compressible::compression_info::CompressionInfo;

use crate::{
    BASE_TOKEN_ACCOUNT_SIZE, EXTENSION_METADATA, TRANSFER_FEE_ACCOUNT_EXTENSION_LEN,
    TRANSFER_HOOK_ACCOUNT_EXTENSION_LEN,
};

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
/// - Base account: 166 bytes (165 SPL token + 1 account_type)
/// - Extension metadata (per extension): 6 bytes (1 Option + 4 Vec len + 1 discriminant)
/// - Compressible: 91 bytes (1 compression_only + 1 decimals + 1 has_decimals + 88 CompressionInfo::LEN)
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
        // CompressibleExtension: 1 compression_only + 1 decimals + 1 has_decimals + CompressionInfo::LEN
        size += 3 + CompressionInfo::LEN as u64 + EXTENSION_METADATA;
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
