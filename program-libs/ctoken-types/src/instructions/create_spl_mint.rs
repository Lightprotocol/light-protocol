use crate::{
    instructions::create_compressed_mint::UpdateCompressedMintInstructionData, AnchorDeserialize,
    AnchorSerialize,
};
use light_zero_copy::ZeroCopy;

#[derive(ZeroCopy, AnchorDeserialize, AnchorSerialize, Clone, Debug)]
pub struct CreateSplMintInstructionData {
    pub mint_bump: u8,
    pub mint: UpdateCompressedMintInstructionData,
    pub mint_authority_is_none: bool, // if mint authority is None anyone can create the spl mint.
}
