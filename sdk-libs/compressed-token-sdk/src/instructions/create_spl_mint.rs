use light_compressed_token_types::ValidityProof;
use light_ctoken_types::instructions::mint_action::CompressedMintWithContext;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::{
    error::Result,
    instructions::mint_action::{create_mint_action, MintActionInputs, MintActionType, TokenPool},
};

pub const POOL_SEED: &[u8] = b"pool";

pub struct CreateSplMintInputs {
    pub mint_signer: Pubkey,
    pub mint_bump: u8,
    pub compressed_mint_inputs: CompressedMintWithContext,
    pub payer: Pubkey,
    pub input_merkle_tree: Pubkey,
    pub input_output_queue: Pubkey,
    pub output_queue: Pubkey,
    pub mint_authority: Pubkey,
    pub proof: ValidityProof,
    pub token_pool: TokenPool,
}

/// Creates an SPL mint instruction using the mint_action instruction as a wrapper
/// This maintains the same API as before but uses mint_action under the hood
pub fn create_spl_mint_instruction(inputs: CreateSplMintInputs) -> Result<Instruction> {
    create_spl_mint_instruction_with_bump(inputs, Pubkey::default(), false)
}

/// Creates an SPL mint instruction with explicit token pool and CPI context options
/// This is now a wrapper around the mint_action instruction
pub fn create_spl_mint_instruction_with_bump(
    inputs: CreateSplMintInputs,
    _token_pool_pda: Pubkey, // Unused in mint_action, kept for API compatibility
    _cpi_context: bool,      // Unused in mint_action, kept for API compatibility
) -> Result<Instruction> {
    let CreateSplMintInputs {
        mint_signer,
        mint_bump,
        compressed_mint_inputs,
        proof,
        payer,
        input_merkle_tree,  // Used for existing compressed mint
        input_output_queue, // Used for existing compressed mint input queue
        output_queue,
        mint_authority,
        token_pool,
    } = inputs;

    // Create the mint_action instruction with CreateSplMint action
    let mint_action_inputs = MintActionInputs {
        compressed_mint_inputs,
        mint_seed: mint_signer,
        create_mint: false, // The compressed mint already exists
        mint_bump: Some(mint_bump),
        authority: mint_authority,
        payer,
        proof: proof.0,
        actions: vec![MintActionType::CreateSplMint { mint_bump }],
        // Use input_merkle_tree since we're operating on existing compressed mint
        address_tree_pubkey: input_merkle_tree,
        input_queue: Some(input_output_queue), // Input queue for existing compressed mint
        output_queue,
        tokens_out_queue: None, // No tokens being minted in CreateSplMint
        token_pool: Some(token_pool), // Required for CreateSplMint action
    };

    create_mint_action(mint_action_inputs)
}
