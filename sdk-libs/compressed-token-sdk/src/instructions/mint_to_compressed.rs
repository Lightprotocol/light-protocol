use light_compressed_token_types::CPI_AUTHORITY_PDA;
use light_ctoken_types::{
    instructions::{
        create_compressed_mint::UpdateCompressedMintInstructionData,
        mint_to_compressed::{CompressedMintInputs, MintToCompressedInstructionData, Recipient},
    },
    COMPRESSED_TOKEN_PROGRAM_ID,
};
use light_sdk::constants::{
    ACCOUNT_COMPRESSION_AUTHORITY_PDA, ACCOUNT_COMPRESSION_PROGRAM_ID, LIGHT_SYSTEM_PROGRAM_ID,
    NOOP_PROGRAM_ID, REGISTERED_PROGRAM_PDA, SOL_POOL_PDA,
};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::{
    error::{Result, TokenSdkError},
    AnchorSerialize,
};

/// Configuration for decompressed mint operations
#[derive(Debug, Clone)]
pub struct DecompressedMintConfig {
    /// SPL mint account
    pub mint_pda: Pubkey,
    /// Token pool PDA
    pub token_pool_pda: Pubkey,
    /// Token program (typically spl_token_2022::ID)
    pub token_program: Pubkey,
}

/// Input parameters for creating a mint_to_compressed instruction
#[derive(Debug, Clone)]
pub struct MintToCompressedInputs {
    pub compressed_mint_inputs: CompressedMintInputs,
    pub lamports: Option<u64>,
    pub recipients: Vec<Recipient>,
    pub mint_authority: Pubkey,
    pub payer: Pubkey,
    pub state_merkle_tree: Pubkey,
    pub output_queue: Pubkey,
    pub state_tree_pubkey: Pubkey,

    /// Required if the mint is decompressed
    pub decompressed_mint_config: Option<DecompressedMintConfig>,
}

/// Create a mint_to_compressed instruction
pub fn create_mint_to_compressed_instruction(
    inputs: MintToCompressedInputs,
) -> Result<Instruction> {
    let MintToCompressedInputs {
        compressed_mint_inputs,
        lamports,
        recipients,
        mint_authority,
        payer,
        state_merkle_tree,
        output_queue,
        state_tree_pubkey,
        decompressed_mint_config,
    } = inputs;

    // Validate that decompressed_mint_config is provided when the mint is decompressed
    if compressed_mint_inputs.compressed_mint_input.is_decompressed
        && decompressed_mint_config.is_none()
    {
        return Err(TokenSdkError::DecompressedMintConfigRequired);
    }

    // Create UpdateCompressedMintInstructionData from CompressedMintInputs
    let update_mint_data = UpdateCompressedMintInstructionData {
        leaf_index: compressed_mint_inputs.leaf_index.into(),
        prove_by_index: compressed_mint_inputs.prove_by_index.into(),
        root_index: compressed_mint_inputs.root_index,
        address: compressed_mint_inputs.address,
        proof: None, // No proof needed for this test
        mint: compressed_mint_inputs.compressed_mint_input.try_into()?,
    };

    // Create mint_to_compressed instruction
    let mint_to_instruction_data = MintToCompressedInstructionData {
        token_account_version: 2, // V2 for batched merkle trees
        compressed_mint_inputs: update_mint_data,
        lamports,
        recipients,
        proof: None, // No proof needed for this test
    };

    // Create accounts in the correct order for manual parsing
    let mut mint_to_accounts = vec![
        // Static non-CPI accounts first
        AccountMeta::new_readonly(mint_authority, true), // 0: authority (signer)
    ];

    // Add decompressed mint accounts if provided
    if let Some(decompressed_config) = &decompressed_mint_config {
        mint_to_accounts.extend([
            AccountMeta::new(decompressed_config.mint_pda, false), // mint
            AccountMeta::new(decompressed_config.token_pool_pda, false), // token_pool_pda
            AccountMeta::new_readonly(decompressed_config.token_program, false), // token_program
        ]);
    }

    mint_to_accounts.extend([
        AccountMeta::new_readonly(LIGHT_SYSTEM_PROGRAM_ID.into(), false), // light_system_program
        // CPI accounts in exact order expected by InvokeCpiWithReadOnly
        AccountMeta::new(payer, true), // fee_payer (signer, mutable)
        AccountMeta::new_readonly(CPI_AUTHORITY_PDA.into(), false), // cpi_authority_pda
        AccountMeta::new_readonly(REGISTERED_PROGRAM_PDA.into(), false), // registered_program_pda
        AccountMeta::new_readonly(NOOP_PROGRAM_ID.into(), false), // noop_program
        AccountMeta::new_readonly(ACCOUNT_COMPRESSION_AUTHORITY_PDA.into(), false), // account_compression_authority
        AccountMeta::new_readonly(ACCOUNT_COMPRESSION_PROGRAM_ID.into(), false), // account_compression_program
        AccountMeta::new_readonly(Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID), false), // self_program
    ]);

    if inputs.lamports.is_some() {
        mint_to_accounts.push(AccountMeta::new(SOL_POOL_PDA.into(), false)); // sol_pool_pda (mutable)
    }
    mint_to_accounts.push(AccountMeta::new_readonly(
        solana_pubkey::Pubkey::default(),
        false,
    )); // system_program
    mint_to_accounts.extend([
        AccountMeta::new(state_merkle_tree, false), // mint_merkle_tree (mutable)
        AccountMeta::new(output_queue, false),      // mint_in_queue (mutable)
        AccountMeta::new(output_queue, false),      // mint_out_queue (mutable)
        AccountMeta::new(output_queue, false),      // tokens_out_queue (mutable)
    ]);

    // Add remaining accounts: compressed mint's address tree, then output state tree
    mint_to_accounts.push(AccountMeta::new(state_tree_pubkey, false)); // Compressed mint's queue

    let instruction = Instruction {
        program_id: Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: mint_to_accounts,
        data: [
            vec![101], // mint_to_compressed discriminator
            mint_to_instruction_data
                .try_to_vec()
                .map_err(|_| TokenSdkError::SerializationError)?,
        ]
        .concat(),
    };

    Ok(instruction)
}
