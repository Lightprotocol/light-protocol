use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize};

/// CompressedOnly extension for compressed token accounts.
/// This extension marks a compressed account as decompress-only (cannot be transferred).
/// It stores the delegated amount from the source CToken account when it was compressed-and-closed.
#[derive(
    Debug,
    Clone,
    Hash,
    Copy,
    PartialEq,
    Eq,
    AnchorSerialize,
    AnchorDeserialize,
    ZeroCopy,
    ZeroCopyMut,
)]
#[repr(C)]
pub struct CompressedOnlyExtension {
    /// The delegated amount from the source CToken account's delegate field.
    /// When decompressing, the decompression amount must match this value.
    pub delegated_amount: u64,
    /// Withheld transfer fee amount from the source CToken account.
    pub withheld_transfer_fee: u64,
    /// Whether the source was an ATA (1) or regular token account (0).
    /// When is_ata=1, decompress must verify ATA derivation matches.
    pub is_ata: u8,
}

impl CompressedOnlyExtension {
    pub const LEN: usize = 17;
}
