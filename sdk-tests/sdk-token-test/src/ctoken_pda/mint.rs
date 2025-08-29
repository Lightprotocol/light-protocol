use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_compressed_token_sdk::instructions::{
    mint_action::{MintActionCpiWriteAccounts, MintActionType},
    mint_action_cpi_write, MintActionInputsCpiWrite,
};
use light_sdk::cpi::CpiAccountsSmall;

use super::CTokenPda;
use crate::ChainedCtokenInstructionData;

pub fn process_mint_action<'c, 'info>(
    ctx: &Context<'_, '_, 'c, 'info, CTokenPda<'info>>,
    input: &ChainedCtokenInstructionData,
    cpi_accounts: &CpiAccountsSmall<'c, 'info>,
) -> Result<()> {
    let actions = vec![
        MintActionType::MintTo {
            recipients: input.token_recipients.clone(),
            lamports: input.lamports,
            token_account_version: 2,
        },
        MintActionType::UpdateMintAuthority {
            new_authority: input.final_mint_authority,
        },
    ];

    let mint_action_inputs = MintActionInputsCpiWrite {
        compressed_mint_inputs: input.compressed_mint_with_context.clone(),
        mint_seed: Some(ctx.accounts.mint_seed.key()),
        mint_bump: Some(input.mint_bump),
        create_mint: true,
        authority: ctx.accounts.mint_authority.key(),
        payer: ctx.accounts.payer.key(),
        actions,
        input_queue: None, // Not needed for create_mint: true
        cpi_context: light_ctoken_types::instructions::mint_action::CpiContext {
            set_context: false,
            first_set_context: true,
            in_tree_index: 0,
            in_queue_index: 1,
            out_queue_index: 1,
            token_out_queue_index: 1,
            assigned_account_index: 0,
        },
        cpi_context_pubkey: *cpi_accounts.cpi_context().unwrap().key,
    };

    let mint_action_instruction = mint_action_cpi_write(mint_action_inputs).unwrap();
    let mint_action_account_infos = MintActionCpiWriteAccounts {
        light_system_program: cpi_accounts.system_program().unwrap(),
        mint_signer: Some(ctx.accounts.mint_seed.as_ref()),
        authority: ctx.accounts.mint_authority.as_ref(),
        fee_payer: ctx.accounts.payer.as_ref(),
        cpi_authority_pda: ctx.accounts.ctoken_cpi_authority.as_ref(),
        cpi_context: cpi_accounts.cpi_context().unwrap(),
        cpi_signer: crate::LIGHT_CPI_SIGNER,
        recipient_token_accounts: vec![],
    };

    invoke(
        &mint_action_instruction,
        &mint_action_account_infos.to_account_infos(),
    )?;

    Ok(())
}
