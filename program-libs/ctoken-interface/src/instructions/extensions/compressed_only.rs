use light_zero_copy::ZeroCopy;

use crate::{AnchorDeserialize, AnchorSerialize};

/// CompressedOnly extension instruction data for compressed token accounts.
/// This extension marks a compressed account as decompress-only (cannot be transferred).
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
#[repr(C)]
pub struct CompressedOnlyExtensionInstructionData {
    /// The delegated amount from the source CToken account's delegate field.
    /// When decompressing, the decompression amount must match this value.
    pub delegated_amount: u64,
    /// Withheld transfer fee amount
    pub withheld_transfer_fee: u64,
    /// Whether the source CToken account was frozen when compressed.
    pub is_frozen: bool,
}
