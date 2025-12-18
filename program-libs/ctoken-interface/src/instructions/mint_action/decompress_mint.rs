use light_zero_copy::ZeroCopy;

use crate::{AnchorDeserialize, AnchorSerialize};

/// Action to decompress a compressed mint to a CMint Solana account.
/// Creates a CMint PDA that becomes the source of truth for the mint state.
///
/// CMint is ALWAYS compressible - `rent_payment` must be >= 2.
/// rent_payment == 0 or 1 is rejected (epoch boundary edge case).
#[repr(C)]
#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct DecompressMintAction {
    /// PDA bump for CMint account verification
    pub cmint_bump: u8,
    /// Rent payment in epochs (prepaid). REQUIRED field.
    /// CMint is ALWAYS compressible - must be >= 2.
    /// NOTE: rent_payment == 0 or 1 is REJECTED.
    pub rent_payment: u8,
    /// Lamports allocated for future write operations (top-up per write).
    /// Must not exceed config.rent_config.max_top_up.
    pub write_top_up: u32,
}
