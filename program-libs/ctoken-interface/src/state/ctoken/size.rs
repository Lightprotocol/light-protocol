use crate::{
    BASE_TOKEN_ACCOUNT_SIZE, EXTENSION_METADATA, TRANSFER_FEE_ACCOUNT_EXTENSION_LEN,
    TRANSFER_HOOK_ACCOUNT_EXTENSION_LEN,
};

/// Calculates the size of a ctoken account based on which extensions are present.
///
/// Note: Compression info is now embedded in the base struct (CTokenZeroCopyMeta),
/// so there's no separate compressible extension parameter.
///
/// # Arguments
/// * `has_pausable` - Whether the account has the PausableAccount extension (marker, 0 bytes)
/// * `has_permanent_delegate` - Whether the account has the PermanentDelegateAccount extension (marker, 0 bytes)
/// * `has_transfer_fee` - Whether the account has the TransferFeeAccount extension (8 bytes)
/// * `has_transfer_hook` - Whether the account has the TransferHookAccount extension (1 byte transferring)
///
/// # Returns
/// The total account size in bytes
///
/// # Extension Sizes
/// - Base account: 258 bytes (165 SPL token + 1 account_type + 2 decimals + 1 compression_only + 88 CompressionInfo + 1 has_extensions)
/// - Extension metadata: 5 bytes (1 Option discriminator + 4 Vec length) - added when any extension present
/// - PausableAccount: 1 byte (discriminant only, marker extension)
/// - PermanentDelegateAccount: 1 byte (discriminant only, marker extension)
/// - TransferFeeAccount: 9 bytes (1 discriminant + 8 withheld_amount)
/// - TransferHookAccount: 2 bytes (1 discriminant + 1 transferring flag)
pub const fn calculate_ctoken_account_size(
    has_pausable: bool,
    has_permanent_delegate: bool,
    has_transfer_fee: bool,
    has_transfer_hook: bool,
) -> u64 {
    let has_any_extension = has_pausable || has_permanent_delegate || has_transfer_fee || has_transfer_hook;

    let mut size = BASE_TOKEN_ACCOUNT_SIZE;

    // Add extension metadata overhead if any extensions are present
    if has_any_extension {
        size += EXTENSION_METADATA;
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
