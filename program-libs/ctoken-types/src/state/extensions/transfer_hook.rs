use light_zero_copy::{
    errors::ZeroCopyError,
    traits::{ZeroCopyAt, ZeroCopyAtMut},
    ZeroCopyNew,
};

use crate::{AnchorDeserialize, AnchorSerialize};

/// Extension indicating the account belongs to a mint with transfer hook.
/// Contains a `transferring` flag used as a reentrancy guard during hook CPI.
/// Consistent with SPL Token-2022 TransferHookAccount layout.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default, AnchorSerialize, AnchorDeserialize)]
#[repr(C)]
pub struct TransferHookAccountExtension {
    /// Flag to indicate that the account is in the middle of a transfer.
    /// Used as reentrancy guard when transfer hook program is called via CPI.
    /// Always false at rest since we only support nil program_id (no hook invoked).
    pub transferring: u8,
}

/// Zero-copy reference for TransferHookAccountExtension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ZTransferHookAccountExtension {
    pub transferring: u8,
}

/// Zero-copy mutable reference for TransferHookAccountExtension.
#[derive(Debug)]
pub struct ZTransferHookAccountExtensionMut<'a> {
    pub transferring: &'a mut u8,
}

impl<'a> ZeroCopyAt<'a> for TransferHookAccountExtension {
    type ZeroCopyAt = ZTransferHookAccountExtension;

    fn zero_copy_at(data: &'a [u8]) -> Result<(Self::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
        if data.is_empty() {
            return Err(ZeroCopyError::Size);
        }
        let transferring = data[0];
        Ok((ZTransferHookAccountExtension { transferring }, &data[1..]))
    }
}

impl<'a> ZeroCopyAtMut<'a> for TransferHookAccountExtension {
    type ZeroCopyAtMut = ZTransferHookAccountExtensionMut<'a>;

    fn zero_copy_at_mut(
        data: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
        if data.is_empty() {
            return Err(ZeroCopyError::Size);
        }
        let (transferring_byte, rest) = data.split_at_mut(1);
        Ok((
            ZTransferHookAccountExtensionMut {
                transferring: &mut transferring_byte[0],
            },
            rest,
        ))
    }
}

/// Config for TransferHookAccountExtension initialization.
/// Empty since transferring is always initialized to false.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TransferHookAccountExtensionConfig;

impl<'a> ZeroCopyNew<'a> for TransferHookAccountExtension {
    type ZeroCopyConfig = TransferHookAccountExtensionConfig;
    type Output = ZTransferHookAccountExtensionMut<'a>;

    fn byte_len(_config: &Self::ZeroCopyConfig) -> Result<usize, ZeroCopyError> {
        // 1 byte for transferring flag
        Ok(1)
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        _config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        if bytes.is_empty() {
            return Err(ZeroCopyError::Size);
        }
        let (transferring_byte, rest) = bytes.split_at_mut(1);
        // Initialize to false (0)
        transferring_byte[0] = 0;
        Ok((
            ZTransferHookAccountExtensionMut {
                transferring: &mut transferring_byte[0],
            },
            rest,
        ))
    }
}
