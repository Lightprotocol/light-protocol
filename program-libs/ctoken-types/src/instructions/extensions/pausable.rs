use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize};

/// Instruction data for PausableAccount extension.
/// Contains the mint_index which is used at runtime to check pausable status,
/// but is NOT persisted to the account (stripped when copying to account data).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
#[repr(C)]
pub struct PausableExtensionInstructionData {
    /// Index of the mint account in packed accounts.
    /// Used to check if the SPL mint has PausableConfig and is paused.
    /// This field is stripped when persisting to account data since
    /// PausableAccount is a marker extension with no persisted data.
    pub mint_index: u8,
}
