use light_compressed_account::Pubkey;
use light_zero_copy::ZeroCopy;

use crate::{
    instructions::extensions::compressible::CompressibleExtensionInstructionData,
    AnchorDeserialize, AnchorSerialize,
};

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct CreateTokenAccountInstructionData {
    /// The owner of the token account
    pub owner: Pubkey,
    /// Optional compressible configuration for the token account
    pub compressible_config: Option<CompressibleExtensionInstructionData>,
}
