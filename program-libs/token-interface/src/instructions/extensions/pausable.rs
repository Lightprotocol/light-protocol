use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize};

/// Instruction data for PausableAccount extension.
/// PausableAccount is a marker extension with no persisted data.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
#[repr(C)]
pub struct PausableExtensionInstructionData;
