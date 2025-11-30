use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize};

/// Instruction data for PermanentDelegateAccount extension.
/// Contains the mint_index which is used at runtime to check permanent delegate status,
/// but is NOT persisted to the account (stripped when copying to account data).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
#[repr(C)]
pub struct PermanentDelegateExtensionInstructionData {
    /// Index of the mint account in packed accounts.
    /// Used to check if the SPL mint has PermanentDelegate extension and get the delegate pubkey.
    /// This field is stripped when persisting to account data since
    /// PermanentDelegateAccount is a marker extension with no persisted data.
    pub mint_index: u8,
}
