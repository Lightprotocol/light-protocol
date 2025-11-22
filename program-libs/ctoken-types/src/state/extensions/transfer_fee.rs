use light_zero_copy::{
    errors::ZeroCopyError,
    traits::{ZeroCopyAt, ZeroCopyAtMut},
    ZeroCopyNew,
};

use crate::{AnchorDeserialize, AnchorSerialize};

/// Transfer fee extension for CToken accounts.
/// Stores withheld fees that accumulate during transfers.
/// Mirrors SPL Token-2022's TransferFeeAmount extension.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default, AnchorSerialize, AnchorDeserialize)]
#[repr(C)]
pub struct TransferFeeAccountExtension {
    /// Amount withheld during transfers, to be harvested on decompress
    pub withheld_amount: u64,
}

impl TransferFeeAccountExtension {
    pub const LEN: usize = 8; // u64
}

/// Zero-copy reference for TransferFeeAccountExtension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ZTransferFeeAccountExtension {
    pub withheld_amount: u64,
}

/// Zero-copy mutable reference for TransferFeeAccountExtension.
#[derive(Debug)]
pub struct ZTransferFeeAccountExtensionMut<'a> {
    data: &'a mut [u8],
}

impl<'a> ZTransferFeeAccountExtensionMut<'a> {
    pub fn withheld_amount(&self) -> u64 {
        u64::from_le_bytes(self.data[0..8].try_into().unwrap())
    }

    pub fn set_withheld_amount(&mut self, amount: u64) {
        self.data[0..8].copy_from_slice(&amount.to_le_bytes());
    }

    /// Add fee to withheld amount (used during transfers).
    /// Returns error if addition would overflow.
    pub fn add_withheld_amount(&mut self, fee: u64) -> Result<(), ArithmeticOverflow> {
        let current = self.withheld_amount();
        let new_amount = current.checked_add(fee).ok_or(ArithmeticOverflow)?;
        self.set_withheld_amount(new_amount);
        Ok(())
    }
}

/// Error returned when arithmetic operation overflows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArithmeticOverflow;

impl<'a> ZeroCopyAt<'a> for TransferFeeAccountExtension {
    type ZeroCopyAt = ZTransferFeeAccountExtension;

    fn zero_copy_at(data: &'a [u8]) -> Result<(Self::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
        if data.len() < Self::LEN {
            return Err(ZeroCopyError::ArraySize(Self::LEN, data.len()));
        }
        let withheld_amount = u64::from_le_bytes(data[0..8].try_into().unwrap());
        Ok((
            ZTransferFeeAccountExtension { withheld_amount },
            &data[Self::LEN..],
        ))
    }
}

impl<'a> ZeroCopyAtMut<'a> for TransferFeeAccountExtension {
    type ZeroCopyAtMut = ZTransferFeeAccountExtensionMut<'a>;

    fn zero_copy_at_mut(
        data: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
        if data.len() < Self::LEN {
            return Err(ZeroCopyError::ArraySize(Self::LEN, data.len()));
        }
        let (ext_data, remaining) = data.split_at_mut(Self::LEN);
        Ok((
            ZTransferFeeAccountExtensionMut { data: ext_data },
            remaining,
        ))
    }
}

/// Config for TransferFeeAccountExtension initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TransferFeeAccountExtensionConfig;

impl<'a> ZeroCopyNew<'a> for TransferFeeAccountExtension {
    type ZeroCopyConfig = TransferFeeAccountExtensionConfig;
    type Output = ZTransferFeeAccountExtensionMut<'a>;

    fn byte_len(_config: &Self::ZeroCopyConfig) -> Result<usize, ZeroCopyError> {
        Ok(Self::LEN)
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        _config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        if bytes.len() < Self::LEN {
            return Err(ZeroCopyError::ArraySize(Self::LEN, bytes.len()));
        }
        // withheld_amount is already zero-initialized, no need to write zeros
        let (ext_data, remaining) = bytes.split_at_mut(Self::LEN);
        Ok((
            ZTransferFeeAccountExtensionMut { data: ext_data },
            remaining,
        ))
    }
}
