use light_zero_copy::ZeroCopy;

use crate::{
    instructions::create_ctoken_account::CompressToPubkey, AnchorDeserialize, AnchorSerialize,
};

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct CreateAssociatedTokenAccountInstructionData {
    pub bump: u8,
    /// Version of the compressed token account when ctoken account is
    /// compressed and closed. (The version specifies the hashing scheme.)
    pub token_account_version: u8,
    /// Rent payment in epochs.
    /// Paid once at initialization.
    pub rent_payment: u8,
    /// If true, the compressed token account cannot be transferred,
    /// only decompressed. Used for delegated compress operations.
    pub compression_only: u8,
    pub write_top_up: u32,
    /// Optional compressible configuration for the token account
    pub compressible_config: Option<CompressToPubkey>,
}
