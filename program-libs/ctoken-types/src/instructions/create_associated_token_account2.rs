use light_zero_copy::ZeroCopy;

use crate::{
    instructions::extensions::compressible::CompressibleExtensionInstructionData,
    AnchorDeserialize, AnchorSerialize,
};

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct CreateAssociatedTokenAccount2InstructionData {
    pub bump: u8,
    /// Optional compressible configuration for the token account
    pub compressible_config: Option<CompressibleExtensionInstructionData>,
}
