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
    /// Index of the compression operation that consumes this input.
    pub compression_index: u8,
    /// Whether the source CToken account was an ATA.
    /// When is_ata=true, decompress must verify ATA derivation matches.
    pub is_ata: bool,
    /// ATA derivation bump (only used when is_ata=true).
    pub bump: u8,
    /// Index into packed_accounts for the actual owner (only used when is_ata=true).
    /// For ATA decompress: this is the wallet owner who signs. The program derives
    /// ATA from (owner, program_id, mint, bump) and verifies it matches the
    /// compressed account owner (which is the ATA pubkey).
    pub owner_index: u8,
}
