use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_zero_copy::ZeroCopy;

use crate::{
    instructions::create_compressed_mint::CompressedMintWithContext, AnchorDeserialize,
    AnchorSerialize,
};

#[repr(C)]
#[derive(ZeroCopy, AnchorDeserialize, AnchorSerialize, Clone, Debug)]
pub struct CreateSplMintInstructionData {
    pub mint_bump: u8,
    pub mint_authority_is_none: bool, // if mint authority is None anyone can create the spl mint.
    pub cpi_context: bool,            // Can only execute since mutates solana account state.
    pub mint: CompressedMintWithContext,
    pub proof: Option<CompressedProof>,
}
