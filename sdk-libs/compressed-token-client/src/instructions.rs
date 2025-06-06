//! Instruction builders for compressed token operations

use anchor_spl::token_interface::spl_token_2022;
use light_compressed_token::{process_transfer::TokenTransferOutputData, TokenData};
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;

use crate::{transfer_sdk, CompressedAccount, CompressedProof, MerkleContext};

/// Error type for instruction builder operations
#[derive(Debug, thiserror::Error)]
pub enum CompressedTokenError {
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Parameters for creating a compress instruction
#[derive(Debug, Clone)]
pub struct CompressParams {
    /// The payer of the transaction
    pub payer: Pubkey,
    /// Owner of the uncompressed token account
    pub owner: Pubkey,
    /// Source token account address
    pub source: Pubkey,
    /// Owner of the compressed token account
    pub to_address: Pubkey,
    /// Mint address of the token to compress
    pub mint: Pubkey,
    /// Amount of tokens to compress
    pub amount: u64,
    /// The state tree that the output should be inserted into
    pub output_state_tree: Pubkey,
    /// Optional: The token program ID. Default: SPL Token Program ID
    pub token_program_id: Option<Pubkey>,
    /// Optional: Multiple recipients and amounts for batch compression
    pub batch_recipients: Option<Vec<(Pubkey, u64)>>,
}

/// Parameters for creating a decompress instruction
#[derive(Debug, Clone)]
pub struct DecompressParams {
    /// The payer of the transaction
    pub payer: Pubkey,
    /// Input compressed token accounts to be consumed
    pub input_compressed_token_accounts: Vec<(CompressedAccount, TokenData, MerkleContext)>,
    /// Address of uncompressed destination token account
    pub to_address: Pubkey,
    /// Amount of tokens to decompress
    pub amount: u64,
    /// The recent state root indices of the input state
    pub recent_input_state_root_indices: Vec<Option<u16>>,
    /// The recent validity proof for state inclusion
    pub recent_validity_proof: Option<CompressedProof>,
    /// The state tree that the change output should be inserted into
    pub output_state_tree: Option<Pubkey>,
    /// Optional: The token program ID. Default: SPL Token Program ID
    pub token_program_id: Option<Pubkey>,
}

/// Create a compress instruction
///
/// This instruction transfers tokens from an SPL token account to compressed token accounts.
pub fn create_compress_instruction(
    params: CompressParams,
) -> Result<Instruction, CompressedTokenError> {
    let token_program = params.token_program_id.unwrap_or(anchor_spl::token::ID);

    // Create output compressed accounts
    let output_compressed_accounts = if let Some(batch_recipients) = params.batch_recipients {
        batch_recipients
            .into_iter()
            .map(|(recipient, amount)| TokenTransferOutputData {
                owner: recipient,
                amount,
                lamports: None,
                merkle_tree: params.output_state_tree,
            })
            .collect()
    } else {
        vec![TokenTransferOutputData {
            owner: params.to_address,
            amount: params.amount,
            lamports: None,
            merkle_tree: params.output_state_tree,
        }]
    };

    // Calculate total amount
    let total_amount: u64 = output_compressed_accounts.iter().map(|x| x.amount).sum();

    // Create the instruction using the transfer SDK
    transfer_sdk::create_transfer_instruction(
        &params.payer,
        &params.owner,
        &[], // empty input merkle context for compression
        &output_compressed_accounts,
        &[],   // empty root indices for compression
        &None, // no proof needed for compression
        &[],   // empty input token data for compression
        &[],   // empty input compressed accounts for compression
        params.mint,
        None,                                          // no delegate
        true,                                          // is_compress = true
        Some(total_amount),                            // compress_or_decompress_amount
        Some(crate::get_token_pool_pda(&params.mint)), // token_pool_pda
        Some(params.source),                           // compress_or_decompress_token_account
        false,                                         // don't sort outputs
        None,                                          // no delegate change account
        None,                                          // no lamports change account
        token_program == spl_token_2022::ID,           // is_token_22
        &[],                                           // no additional token pools
        false,                                         // with_transaction_hash = false
    )
    .map_err(|e| {
        CompressedTokenError::SerializationError(format!("Failed to create instruction: {:?}", e))
    })
}

/// Create a decompress instruction
///
/// This instruction transfers tokens from compressed token accounts to an SPL token account.
pub fn create_decompress_instruction(
    params: DecompressParams,
) -> Result<Instruction, CompressedTokenError> {
    if params.input_compressed_token_accounts.is_empty() {
        return Err(CompressedTokenError::InvalidParams(
            "No input compressed token accounts provided".to_string(),
        ));
    }

    let token_program = params.token_program_id.unwrap_or(anchor_spl::token::ID);

    // Extract components from input accounts
    let (compressed_accounts, token_data, merkle_contexts): (Vec<_>, Vec<_>, Vec<_>) = params
        .input_compressed_token_accounts
        .into_iter()
        .map(|(account, data, context)| (account, data, context))
        .fold(
            (Vec::new(), Vec::new(), Vec::new()),
            |(mut accounts, mut data, mut contexts), (account, token_data, context)| {
                accounts.push(account);
                data.push(token_data);
                contexts.push(context);
                (accounts, data, contexts)
            },
        );

    // Get mint from first token data
    let mint = token_data[0].mint;

    // Get owner from first token data
    let owner = token_data[0].owner;

    // Create output state for remaining tokens (if any)
    let input_total: u64 = token_data.iter().map(|td| td.amount).sum();
    let remaining_amount = input_total.saturating_sub(params.amount);

    let output_compressed_accounts = if remaining_amount > 0 {
        vec![TokenTransferOutputData {
            owner,
            amount: remaining_amount,
            lamports: None,
            merkle_tree: params
                .output_state_tree
                .unwrap_or(merkle_contexts[0].merkle_tree_pubkey),
        }]
    } else {
        vec![]
    };

    // Create the instruction using the transfer SDK
    transfer_sdk::create_transfer_instruction(
        &params.payer,
        &owner,
        &merkle_contexts,
        &output_compressed_accounts,
        &params.recent_input_state_root_indices,
        &params.recent_validity_proof,
        &token_data,
        &compressed_accounts,
        mint,
        None,                                   // no delegate
        false,                                  // is_compress = false
        Some(params.amount),                    // compress_or_decompress_amount
        Some(crate::get_token_pool_pda(&mint)), // token_pool_pda
        Some(params.to_address),                // compress_or_decompress_token_account
        false,                                  // don't sort outputs
        None,                                   // no delegate change account
        None,                                   // no lamports change account
        token_program == spl_token_2022::ID,    // is_token_22
        &[],                                    // no additional token pools
        false,                                  // with_transaction_hash = false
    )
    .map_err(|e| {
        CompressedTokenError::SerializationError(format!("Failed to create instruction: {:?}", e))
    })
}

/// Helper function to create a simple compress instruction
///
/// This is a convenience function for the most common compress use case.
pub fn compress(
    payer: Pubkey,
    owner: Pubkey,
    source_token_account: Pubkey,
    mint: Pubkey,
    amount: u64,
    to_address: Pubkey,
    output_state_tree: Pubkey,
) -> Result<Instruction, CompressedTokenError> {
    create_compress_instruction(CompressParams {
        payer,
        owner,
        source: source_token_account,
        to_address,
        mint,
        amount,
        output_state_tree,
        token_program_id: None,
        batch_recipients: None,
    })
}

/// Helper function to create a batch compress instruction
///
/// Compress tokens to multiple recipients in a single transaction.
pub fn batch_compress(
    payer: Pubkey,
    owner: Pubkey,
    source_token_account: Pubkey,
    mint: Pubkey,
    recipients: Vec<(Pubkey, u64)>,
    output_state_tree: Pubkey,
) -> Result<Instruction, CompressedTokenError> {
    create_compress_instruction(CompressParams {
        payer,
        owner,
        source: source_token_account,
        to_address: Pubkey::default(), // Not used in batch mode
        mint,
        amount: 0, // Not used in batch mode
        output_state_tree,
        token_program_id: None,
        batch_recipients: Some(recipients),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PROGRAM_ID;

    #[test]
    fn test_compress_instruction() {
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let source = Pubkey::new_unique();
        let to_address = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let output_state_tree = Pubkey::new_unique();

        let result = compress(
            payer,
            owner,
            source,
            mint,
            1000,
            to_address,
            output_state_tree,
        );

        assert!(result.is_ok());
        let instruction = result.unwrap();
        assert_eq!(instruction.program_id, PROGRAM_ID);
    }

    #[test]
    fn test_batch_compress_instruction() {
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let source = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let output_state_tree = Pubkey::new_unique();

        let recipients = vec![
            (Pubkey::new_unique(), 500),
            (Pubkey::new_unique(), 300),
            (Pubkey::new_unique(), 200),
        ];

        let result = batch_compress(payer, owner, source, mint, recipients, output_state_tree);

        assert!(result.is_ok());
        let instruction = result.unwrap();
        assert_eq!(instruction.program_id, PROGRAM_ID);
    }
}
