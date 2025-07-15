use light_zero_copy::ZeroCopy;
use crate::{AnchorSerialize, AnchorDeserialize, instructions::create_compressed_mint::UpdateCompressedMintInstructionData};

#[derive(ZeroCopy, AnchorDeserialize, AnchorSerialize, Clone, Debug)]
pub struct CreateSplMintInstructionData {
    pub mint_bump: u8,
    pub mint: UpdateCompressedMintInstructionData,
}
