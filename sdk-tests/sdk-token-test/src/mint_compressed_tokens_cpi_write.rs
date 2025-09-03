use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_compressed_token_sdk::instructions::{
    mint_action::{MintActionCpiWriteAccounts, MintActionType},
    mint_action_cpi_write, MintActionInputsCpiWrite, MintToRecipient,
};
use light_ctoken_types::instructions::mint_action::CompressedMintWithContext;
use light_sdk::cpi::CpiAccountsSmall;

use crate::Generic;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct MintCompressedTokensCpiWriteParams {
    pub compressed_mint_with_context: CompressedMintWithContext,
    pub recipients: Vec<MintToRecipient>,
    pub cpi_context: light_ctoken_types::instructions::mint_action::CpiContext,
    pub cpi_context_pubkey: Pubkey,
}

/// Process minting compressed tokens to an existing mint using CPI write
/// This sets up the CPI context for subsequent operations
pub fn process_mint_compressed_tokens_cpi_write<'info>(
    ctx: &Context<'_, '_, '_, 'info, Generic<'info>>,
    params: MintCompressedTokensCpiWriteParams,
    token_program_cpi_authority: &AccountInfo<'info>,
    cpi_accounts: &CpiAccountsSmall<'_, 'info>,
) -> Result<()> {
    msg!("Minting compressed tokens with CPI write");

    let actions = vec![MintActionType::MintTo {
        recipients: params.recipients,
        lamports: None,
        token_account_version: 2,
    }];

    let mint_action_inputs = MintActionInputsCpiWrite {
        compressed_mint_inputs: params.compressed_mint_with_context,
        mint_seed: None,    // Not needed for existing mint
        mint_bump: None,    // Not needed for existing mint
        create_mint: false, // Using existing mint
        authority: ctx.accounts.signer.key(),
        payer: ctx.accounts.signer.key(),
        actions,
        input_queue: None,
        cpi_context: params.cpi_context,
        cpi_context_pubkey: *cpi_accounts.cpi_context().unwrap().key,
    };
    msg!("mint_action_inputs {:?}", mint_action_inputs);

    let mint_action_instruction = mint_action_cpi_write(mint_action_inputs).unwrap();

    let mint_action_account_infos = MintActionCpiWriteAccounts {
        light_system_program: cpi_accounts.light_system_program().unwrap(),
        mint_signer: None, // No mint signer for existing mint
        authority: ctx.accounts.signer.as_ref(),
        fee_payer: ctx.accounts.signer.as_ref(),
        cpi_authority_pda: token_program_cpi_authority,
        cpi_context: cpi_accounts.cpi_context().unwrap(),
        cpi_signer: crate::LIGHT_CPI_SIGNER,
        recipient_token_accounts: vec![],
    };
    msg!(
        "mint_action_account_infos.to_account_infos() {:?}",
        mint_action_account_infos
            .to_account_infos()
            .iter()
            .map(|x| x.key)
            .collect::<Vec<_>>()
    );
    invoke(
        &mint_action_instruction,
        &mint_action_account_infos.to_account_infos(),
    )?;

    msg!("Minting completed, CPI context populated");

    Ok(())
}
