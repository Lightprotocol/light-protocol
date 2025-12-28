use light_zero_copy::ZeroCopy;

use crate::{AnchorDeserialize, AnchorSerialize};

/// Action to compress and close a CMint Solana account.
/// The compressed mint state is always preserved.
///
/// ## Requirements
/// - CMint must exist (cmint_decompressed = true) - unless idempotent is set
/// - CMint must have Compressible extension
/// - is_compressible() must return true (rent expired)
/// - Cannot be combined with DecompressMint in same instruction
///
/// ## Note
/// CompressAndCloseCMint is **permissionless** - anyone can compress and close a CMint
/// provided is_compressible() returns true. All lamports are returned to rent_sponsor.
#[repr(C)]
#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct CompressAndCloseCMintAction {
    /// If non-zero, succeed silently when CMint doesn't exist or cannot be compressed.
    /// Useful for foresters to handle already-compressed mints without failing.
    pub idempotent: u8,
}

impl CompressAndCloseCMintAction {
    /// Returns true if this action should succeed silently when:
    /// - CMint doesn't exist (already compressed)
    /// - CMint cannot be compressed (rent not expired)
    #[inline(always)]
    pub fn is_idempotent(&self) -> bool {
        self.idempotent != 0
    }
}

impl ZCompressAndCloseCMintAction<'_> {
    /// Returns true if this action should succeed silently when:
    /// - CMint doesn't exist (already compressed)
    /// - CMint cannot be compressed (rent not expired)
    #[inline(always)]
    pub fn is_idempotent(&self) -> bool {
        self.idempotent != 0
    }
}
