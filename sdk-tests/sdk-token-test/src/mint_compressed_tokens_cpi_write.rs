use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_ctoken_interface::instructions::mint_action::{
    CompressedMintWithContext, MintActionCompressedInstructionData, MintToCompressedAction,
    Recipient,
};
use light_ctoken_sdk::compressed_token::{
    ctoken_instruction::CTokenInstruction, mint_action::MintActionCpiWriteAccounts,
    transfer2::Transfer2CpiAccounts,
};

use crate::Generic;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct MintCompressedTokensCpiWriteParams {
    pub compressed_mint_with_context: CompressedMintWithContext,
    pub recipients: Vec<Recipient>,
    pub cpi_context: light_ctoken_interface::instructions::mint_action::CpiContext,
    pub cpi_context_pubkey: Pubkey,
}

/// Process minting compressed tokens to an existing mint using CPI write
/// This sets up the CPI context for subsequent operations
pub fn process_mint_compressed_tokens_cpi_write<'info>(
    ctx: &Context<'_, '_, '_, '_, Generic<'info>>,
    params: MintCompressedTokensCpiWriteParams,
    cpi_accounts: &Transfer2CpiAccounts<'_, AccountInfo<'info>>,
) -> Result<()> {
    // Build instruction data using builder pattern
    let instruction_data = MintActionCompressedInstructionData::new(
        params.compressed_mint_with_context,
        None, // No proof for CPI write
    )
    .with_mint_to_compressed(MintToCompressedAction {
        token_account_version: 2,
        recipients: params.recipients,
    })
    .with_cpi_context(params.cpi_context);

    // Build account structure for CPI write
    let mint_action_account_infos = MintActionCpiWriteAccounts {
        authority: ctx.accounts.signer.as_ref(),
        light_system_program: cpi_accounts.light_system_program,
        mint_signer: None, // No mint signer for existing mint
        fee_payer: ctx.accounts.signer.as_ref(),
        cpi_authority_pda: cpi_accounts.compressed_token_cpi_authority,
        cpi_context: cpi_accounts.cpi_context.unwrap(),
        cpi_signer: crate::LIGHT_CPI_SIGNER,
        recipient_token_accounts: vec![],
    };

    // Determine which CPI write method to use based on cpi_context flags
    let mint_action_instruction = if instruction_data
        .cpi_context
        .as_ref()
        .map(|c| c.first_set_context)
        .unwrap_or(false)
    {
        instruction_data.instruction_write_to_cpi_context_first(&mint_action_account_infos)
    } else {
        instruction_data.instruction_write_to_cpi_context_set(&mint_action_account_infos)
    }
    .unwrap();

    invoke(
        &mint_action_instruction,
        &mint_action_account_infos.to_account_infos(),
    )?;

    Ok(())
}
