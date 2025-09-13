use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
#[repr(C)]
pub struct CompressibleExtensionInstructionData {
    /// In Epochs. (could do in slots as well)
    pub rent_payment: u64,
    pub has_top_up: u8,
    pub write_top_up: u32,
}
