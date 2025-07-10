use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::ZeroCopy;

use crate::mint::instructions::UpdateCompressedMintInstructionData;

#[derive(ZeroCopy, BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct CreateSplMintInstructionData {
    pub mint_bump: u8,
    pub mint: UpdateCompressedMintInstructionData,
}
