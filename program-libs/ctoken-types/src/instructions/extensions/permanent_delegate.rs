use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize};

/// Instruction data for PermanentDelegateAccount extension.
/// This is a marker extension - no instruction data needed since
/// the permanent delegate is looked up from the mint at runtime.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
#[repr(C)]
pub struct PermanentDelegateExtensionInstructionData;
