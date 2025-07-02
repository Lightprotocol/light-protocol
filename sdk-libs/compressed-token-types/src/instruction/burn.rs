use borsh::{BorshDeserialize, BorshSerialize};

use crate::instruction::transfer::{
    CompressedCpiContext, CompressedProof, DelegatedTransfer, TokenAccountMeta,
};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct CompressedTokenInstructionDataBurn {
    pub proof: CompressedProof,
    pub input_token_data_with_context: Vec<TokenAccountMeta>,
    pub cpi_context: Option<CompressedCpiContext>,
    pub burn_amount: u64,
    pub change_account_merkle_tree_index: u8,
    pub delegated_transfer: Option<DelegatedTransfer>,
}
