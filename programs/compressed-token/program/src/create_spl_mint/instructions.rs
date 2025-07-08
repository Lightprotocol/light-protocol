use crate::mint_to_compressed::instructions::CompressedMintInputs;
use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::{instruction_data::compressed_proof::CompressedProof, Pubkey};
use light_zero_copy::ZeroCopy;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, ZeroCopy)]
pub struct CreateSplMintInstructionData {
    pub mint_bump: u8,
    pub token_pool_bump: u8,
    // TODO: remove decimals, duplicate input
    pub decimals: u8,
    pub mint_authority: Pubkey,
    pub compressed_mint_inputs: CompressedMintInputs,
    pub freeze_authority: Option<Pubkey>,
    pub proof: Option<CompressedProof>,
}
