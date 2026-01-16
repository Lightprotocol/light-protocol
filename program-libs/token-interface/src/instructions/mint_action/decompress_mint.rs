use light_zero_copy::ZeroCopy;

use crate::{AnchorDeserialize, AnchorSerialize};

/// Action to decompress a compressed mint to a Mint Solana account.
/// Creates a Mint PDA that becomes the source of truth for the mint state.
///
/// Mint is ALWAYS compressible - `rent_payment` must be >= 2.
/// rent_payment == 0 or 1 is rejected (epoch boundary edge case).
#[repr(C)]
#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct DecompressMintAction {
    /// Rent payment in epochs (prepaid).
    pub rent_payment: u8,
    /// Lamports allocated for future write operations (top-up per write).
    /// Must not exceed config.rent_config.max_top_up.
    pub write_top_up: u32,
}
