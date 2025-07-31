use light_compressed_account::{
    instruction_data::{compressed_proof::CompressedProof, cpi_context::CompressedCpiContext},
    Pubkey,
};
use light_zero_copy::ZeroCopy;

use crate::{
    instructions::create_compressed_mint::UpdateCompressedMintInstructionData,
    state::CompressedMint, AnchorDeserialize, AnchorSerialize,
};

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct CompressedMintInputs {
    pub leaf_index: u32,
    pub prove_by_index: bool,
    pub root_index: u16,
    pub address: [u8; 32],
    pub compressed_mint_input: CompressedMint, //TODO: move supply and authority last so that we can send only the hash chain.
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct Recipient {
    pub recipient: Pubkey,
    pub amount: u64,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct MintToCompressedInstructionData {
    pub token_account_version: u8,
    pub compressed_mint_inputs: UpdateCompressedMintInstructionData,
    pub lamports: Option<u64>,
    pub recipients: Vec<Recipient>,
    pub proof: Option<CompressedProof>,
    pub cpi_context: Option<CompressedCpiContext>,
}
