use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke;
use light_compressed_token_sdk::instructions::mint_to_compressed::{
    create_mint_to_compressed_cpi_write, MintToCompressedCpiContextWriteAccounts,
    MintToCompressedInputsCpiWrite,
};
use light_ctoken_types::instructions::{
    create_compressed_mint::CompressedMintWithContext,
    mint_to_compressed::{CpiContext, Recipient},
};
use light_sdk_types::CpiAccountsSmall;

use super::CreateCompressedMint;
use crate::LIGHT_CPI_SIGNER;

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct MintToCompressedInstructionData {
    pub recipients: Vec<Recipient>,
    pub lamports: Option<u64>,
    pub version: u8,
}

pub fn mint_to_compressed<'a, 'b, 'c, 'info>(
    ctx: &Context<'a, 'b, 'c, 'info, CreateCompressedMint<'info>>,
    input: MintToCompressedInstructionData,
    compressed_mint_inputs: CompressedMintWithContext,
    cpi_accounts: &CpiAccountsSmall<'a, AccountInfo<'info>>,
) -> Result<()> {
    let cpi_context_account_info = MintToCompressedCpiContextWriteAccounts {
        mint_authority: ctx.accounts.mint_authority.as_ref(),
        light_system_program: cpi_accounts.system_program().unwrap(),
        fee_payer: ctx.accounts.payer.as_ref(),
        cpi_authority_pda: ctx.accounts.ctoken_cpi_authority.as_ref(),
        cpi_context: cpi_accounts.cpi_context().unwrap(),
        cpi_signer: LIGHT_CPI_SIGNER,
    };

    let mint_to_inputs = MintToCompressedInputsCpiWrite {
        compressed_mint_inputs,
        lamports: input.lamports,
        recipients: input.recipients,
        mint_authority: ctx.accounts.mint_authority.key(),
        payer: ctx.accounts.payer.key(),
        cpi_context: CpiContext {
            set_context: true,
            first_set_context: false,
            in_tree_index: 2,
            in_queue_index: 1,
            out_queue_index: 1,
            token_out_queue_index: 1,
        },
        cpi_context_pubkey: *cpi_accounts.cpi_context().unwrap().key,
        version: input.version,
    };

    let mint_to_instruction =
        create_mint_to_compressed_cpi_write(mint_to_inputs).map_err(ProgramError::from)?;
    // Execute the CPI call to mint compressed tokens
    invoke(
        &mint_to_instruction,
        &cpi_context_account_info.to_account_infos(),
    )?;

    Ok(())
}
