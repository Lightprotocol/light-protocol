use borsh::{BorshDeserialize, BorshSerialize};
use crate::instruction::transfer::{CompressedProof, InputTokenDataWithContext, CompressedCpiContext};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct CompressedTokenInstructionDataApprove {
    pub proof: CompressedProof,
    pub mint: [u8; 32],
    pub input_token_data_with_context: Vec<InputTokenDataWithContext>,
    pub cpi_context: Option<CompressedCpiContext>,
    pub delegate: [u8; 32],
    pub delegated_amount: u64,
    /// Index in remaining accounts.
    pub delegate_merkle_tree_index: u8,
    /// Index in remaining accounts.
    pub change_account_merkle_tree_index: u8,
    pub delegate_lamports: Option<u64>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct CompressedTokenInstructionDataRevoke {
    pub proof: CompressedProof,
    pub mint: [u8; 32],
    pub input_token_data_with_context: Vec<InputTokenDataWithContext>,
    pub cpi_context: Option<CompressedCpiContext>,
    pub output_account_merkle_tree_index: u8,
}