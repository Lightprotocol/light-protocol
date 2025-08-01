use borsh::BorshSerialize;
use light_compressed_token_types::{
    instruction::delegation::CompressedTokenInstructionDataApprove, ValidityProof,
};
use light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::{
    account::CTokenAccount,
    error::{Result, TokenSdkError},
    instructions::approve::account_metas::{
        get_approve_instruction_account_metas, ApproveMetaConfig,
    },
};

#[derive(Debug, Clone)]
pub struct ApproveInputs {
    pub fee_payer: Pubkey,
    pub authority: Pubkey,
    pub sender_account: CTokenAccount,
    pub validity_proof: ValidityProof,
    pub delegate: Pubkey,
    pub delegated_amount: u64,
    pub delegate_lamports: Option<u64>,
    pub delegated_compressed_account_merkle_tree: Pubkey,
    pub change_compressed_account_merkle_tree: Pubkey,
}

/// Create a compressed token approve instruction
/// This creates two output accounts:
/// 1. A delegated account with the specified amount and delegate
/// 2. A change account with the remaining balance (if any)
pub fn create_approve_instruction(inputs: ApproveInputs) -> Result<Instruction> {
    // Store mint before consuming sender_account
    let mint = *inputs.sender_account.mint();
    let (input_token_data, _) = inputs.sender_account.into_inputs_and_outputs();

    if input_token_data.is_empty() {
        return Err(TokenSdkError::InsufficientBalance);
    }

    // Calculate total input amount
    let total_input_amount: u64 = input_token_data.iter().map(|data| data.amount).sum();
    if total_input_amount < inputs.delegated_amount {
        return Err(TokenSdkError::InsufficientBalance);
    }

    // Use the input token data directly since it's already in the correct format
    let input_token_data_with_context = input_token_data;

    // Create instruction data
    let instruction_data = CompressedTokenInstructionDataApprove {
        proof: inputs.validity_proof.0.unwrap(),
        mint: mint.to_bytes(),
        input_token_data_with_context,
        cpi_context: None,
        delegate: inputs.delegate.to_bytes(),
        delegated_amount: inputs.delegated_amount,
        delegate_merkle_tree_index: 0, // Will be set based on remaining accounts
        change_account_merkle_tree_index: 1, // Will be set based on remaining accounts
        delegate_lamports: inputs.delegate_lamports,
    };

    // Serialize instruction data
    let serialized_data = instruction_data
        .try_to_vec()
        .map_err(|_| TokenSdkError::SerializationError)?;

    // Create account meta config
    let meta_config = ApproveMetaConfig::new(
        inputs.fee_payer,
        inputs.authority,
        inputs.delegated_compressed_account_merkle_tree,
        inputs.change_compressed_account_merkle_tree,
    );

    // Get account metas using the dedicated function
    let account_metas = get_approve_instruction_account_metas(meta_config);

    Ok(Instruction {
        program_id: Pubkey::new_from_array(COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: account_metas,
        data: serialized_data,
    })
}

/// Simplified approve function similar to transfer
pub fn approve(inputs: ApproveInputs) -> Result<Instruction> {
    create_approve_instruction(inputs)
}
