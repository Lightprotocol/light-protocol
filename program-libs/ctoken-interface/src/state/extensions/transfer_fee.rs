use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize, CTokenError};

/// Transfer fee extension for CToken accounts.
/// Stores withheld fees that accumulate during transfers.
/// Mirrors SPL Token-2022's TransferFeeAmount extension.
#[derive(
    Debug,
    Clone,
    Copy,
    Hash,
    PartialEq,
    Eq,
    Default,
    AnchorSerialize,
    AnchorDeserialize,
    ZeroCopy,
    ZeroCopyMut,
)]
#[repr(C)]
pub struct TransferFeeAccountExtension {
    /// Amount withheld during transfers, to be harvested on decompress
    pub withheld_amount: u64,
}

impl<'a> ZTransferFeeAccountExtensionMut<'a> {
    /// Add fee to withheld amount (used during transfers).
    /// Returns error if addition would overflow.
    pub fn add_withheld_amount(&mut self, fee: u64) -> Result<(), CTokenError> {
        let current: u64 = self.withheld_amount.get();
        let new_amount = current
            .checked_add(fee)
            .ok_or(CTokenError::ArithmeticOverflow)?;
        self.withheld_amount.set(new_amount);
        Ok(())
    }
}
