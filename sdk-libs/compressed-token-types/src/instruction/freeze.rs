use borsh::{BorshDeserialize, BorshSerialize};

use crate::instruction::transfer::{CompressedCpiContext, CompressedProof, TokenAccountMeta};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct CompressedTokenInstructionDataFreeze {
    pub proof: CompressedProof,
    pub owner: [u8; 32],
    pub input_token_data_with_context: Vec<TokenAccountMeta>,
    pub cpi_context: Option<CompressedCpiContext>,
    pub outputs_merkle_tree_index: u8,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct CompressedTokenInstructionDataThaw {
    pub proof: CompressedProof,
    pub owner: [u8; 32],
    pub input_token_data_with_context: Vec<TokenAccountMeta>,
    pub cpi_context: Option<CompressedCpiContext>,
    pub outputs_merkle_tree_index: u8,
}
