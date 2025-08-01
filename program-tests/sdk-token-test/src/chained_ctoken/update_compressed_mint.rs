use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke;
use light_compressed_token_sdk::instructions::{
    mint_to_compressed::MintToCompressedCpiContextWriteAccounts,
    update_compressed_mint::{
        create_update_compressed_mint_cpi_write, UpdateCompressedMintInputsCpiWrite,
    },
};
use light_ctoken_types::{
    instructions::{
        create_compressed_mint::UpdateCompressedMintInstructionData,
        update_compressed_mint::{CompressedMintAuthorityType, UpdateMintCpiContext},
    },
};
use light_sdk_types::CpiAccountsSmall;

use super::CreateCompressedMint;
use crate::LIGHT_CPI_SIGNER;

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct UpdateCompressedMintInstructionDataCpi {
    pub authority_type: CompressedMintAuthorityType,
    pub new_authority: Option<Pubkey>,
    pub mint_authority: Option<Pubkey>, // Current mint authority (needed when updating freeze authority)
}


pub fn update_compressed_mint_cpi_write<'a, 'b, 'c, 'info>(
    ctx: &Context<'a, 'b, 'c, 'info, CreateCompressedMint<'info>>,
    input: UpdateCompressedMintInstructionDataCpi,
    compressed_mint_inputs: UpdateCompressedMintInstructionData,
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

    // Create CPI context for writing to context (not executing)
    let cpi_context = UpdateMintCpiContext {
        set_context: true,
        first_set_context: false, // This is the third CPI operation
        in_tree_index: 2,
        in_queue_index: 1,
        out_queue_index: 1,
    };

    let update_inputs = UpdateCompressedMintInputsCpiWrite {
        compressed_mint_inputs,
        authority_type: input.authority_type,
        new_authority: input.new_authority,
        mint_authority: input.mint_authority,
        payer: ctx.accounts.payer.key(),
        authority: ctx.accounts.mint_authority.key(),
        cpi_context,
        cpi_context_pubkey: *cpi_accounts.cpi_context().unwrap().key,
    };

    // Create the instruction using the SDK
    let update_instruction = create_update_compressed_mint_cpi_write(update_inputs)
        .map_err(ProgramError::from)?;

    // Execute the CPI call to update compressed mint authority
    invoke(
        &update_instruction,
        &cpi_context_account_info.to_account_infos(),
    )?;

    Ok(())
}