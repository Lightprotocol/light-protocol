use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke;
use light_compressed_token_sdk::account_infos::{
    MintToCompressedAccountInfos, MintToCompressedAccountInfosConfig,
};
use light_compressed_token_sdk::instructions::{
    create_mint_to_compressed_instruction, MintToCompressedInputs,
};
use light_compressed_token_sdk::ValidityProof;
use light_ctoken_types::instructions::mint_to_compressed::{CompressedMintInputs, Recipient};

#[derive(Accounts)]
pub struct MintCompressedTokens<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub mint_authority: Signer<'info>,
}

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct MintCompressedTokensInstructionData {
    pub compressed_mint_inputs: CompressedMintInputs,
    pub recipients: Vec<Recipient>,
    pub lamports: Option<u64>,
    pub validity_proof: ValidityProof,
}

pub fn mint_compressed_tokens<'info>(
    ctx: Context<'_, '_, '_, 'info, MintCompressedTokens<'info>>,
    input: MintCompressedTokensInstructionData,
) -> Result<()> {
    // Determine if SOL pool is needed based on lamports
    let with_sol_pool = input.lamports.is_some();

    // Create the account infos configuration based on input flags
    let account_config = MintToCompressedAccountInfosConfig::new(
        input
            .compressed_mint_inputs
            .compressed_mint_input
            .is_decompressed,
        with_sol_pool,
    );

    // Create the account infos wrapper for CPI use
    let mint_cpi_account_infos = MintToCompressedAccountInfos::new_cpi(
        ctx.accounts.payer.as_ref(),
        ctx.accounts.mint_authority.as_ref(),
        ctx.remaining_accounts,
        account_config,
    );

    // Create decompressed mint config if needed
    let decompressed_mint_config = mint_cpi_account_infos
        .get_decompressed_mint_config()
        .unwrap();

    let mint_to_inputs = MintToCompressedInputs {
        compressed_mint_inputs: input.compressed_mint_inputs,
        lamports: input.lamports,
        recipients: input.recipients,
        mint_authority: ctx.accounts.mint_authority.key(),
        payer: ctx.accounts.payer.key(),
        state_merkle_tree: *mint_cpi_account_infos.in_merkle_tree().unwrap().key,
        output_queue: *mint_cpi_account_infos.out_output_queue().unwrap().key,
        state_tree_pubkey: *mint_cpi_account_infos.tokens_out_queue().unwrap().key,
        decompressed_mint_config,
    };

    let mint_instruction =
        create_mint_to_compressed_instruction(mint_to_inputs).map_err(ProgramError::from)?;

    // Execute the CPI call to mint compressed tokens
    invoke(
        &mint_instruction,
        mint_cpi_account_infos.to_account_infos().as_ref(),
    )?;

    Ok(())
}

#[error_code]
pub enum MintCompressedTokensErrorCode {
    #[msg("Invalid account configuration")]
    InvalidAccountConfiguration,
}
