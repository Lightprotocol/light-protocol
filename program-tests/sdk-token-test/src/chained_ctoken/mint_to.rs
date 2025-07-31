use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke;
use light_compressed_token_sdk::instructions::mint_to_compressed::{
    create_mint_to_compressed_cpi_write, MintToCompressedCpiContextWriteAccounts,
    MintToCompressedInputsCpiWrite,
};
use light_compressed_token_sdk::CompressedCpiContext;
use light_ctoken_types::instructions::mint_to_compressed::{CompressedMintInputs, Recipient};
use light_sdk_types::CpiAccountsSmall;

use super::CreateCompressedMint;
use crate::LIGHT_CPI_SIGNER;

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct MintToCompressedInstructionData {
    pub compressed_mint_inputs: CompressedMintInputs,
    pub recipients: Vec<Recipient>,
    pub lamports: Option<u64>,
    pub version: u8,
}

pub fn mint_to_compressed<'a, 'b, 'c, 'info>(
    ctx: &Context<'a, 'b, 'c, 'info, CreateCompressedMint<'info>>,
    input: MintToCompressedInstructionData,
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
    msg!(" cpi_context_account_info {:?}", cpi_context_account_info);

    let mint_to_inputs = MintToCompressedInputsCpiWrite {
        compressed_mint_inputs: input.compressed_mint_inputs,
        lamports: input.lamports,
        recipients: input.recipients,
        mint_authority: ctx.accounts.mint_authority.key(),
        payer: ctx.accounts.payer.key(),
        cpi_context: CompressedCpiContext {
            set_context: true,
            first_set_context: false,
            cpi_context_account_index: 0,
        },
        cpi_context_pubkey: *cpi_accounts.cpi_context().unwrap().key,
        version: input.version,
    };

    let mint_to_instruction =
        create_mint_to_compressed_cpi_write(mint_to_inputs).map_err(ProgramError::from)?;
    msg!(" mint_to_instruction {:?}", mint_to_instruction);
    // Execute the CPI call to mint compressed tokens
    invoke(
        &mint_to_instruction,
        &cpi_context_account_info.to_account_infos(),
    )?;

    Ok(())
}
