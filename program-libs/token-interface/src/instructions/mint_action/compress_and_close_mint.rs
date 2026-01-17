use light_zero_copy::ZeroCopy;

use crate::{AnchorDeserialize, AnchorSerialize};

/// Action to compress and close a Mint Solana account.
/// The compressed mint state is always preserved.
///
/// ## Requirements
/// - Mint must exist (mint_decompressed = true) - unless idempotent is set
/// - is_compressible() must return true (rent expired)
/// - Cannot be combined with DecompressMint in same instruction
///
/// ## Note
/// CompressAndCloseMint is **permissionless** - anyone can compress and close a Mint
/// provided is_compressible() returns true. All lamports are returned to rent_sponsor.
#[repr(C)]
#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct CompressAndCloseMintAction {
    /// If non-zero, succeed silently when Mint doesn't exist or cannot be compressed.
    /// Useful for foresters to handle already-compressed mints without failing.
    pub idempotent: u8,
}

impl CompressAndCloseMintAction {
    /// Returns true if this action should succeed silently when:
    /// - Mint doesn't exist (already compressed)
    /// - Mint cannot be compressed (rent not expired)
    #[inline(always)]
    pub fn is_idempotent(&self) -> bool {
        self.idempotent != 0
    }
}

impl ZCompressAndCloseMintAction<'_> {
    /// Returns true if this action should succeed silently when:
    /// - Mint doesn't exist (already compressed)
    /// - Mint cannot be compressed (rent not expired)
    #[inline(always)]
    pub fn is_idempotent(&self) -> bool {
        self.idempotent != 0
    }
}
