use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_compressed_token_sdk::instructions::{
    mint_action::{CreateMintCpiWriteInputs, MintActionCpiWriteAccounts, MintActionType},
    mint_action_cpi_write, MintActionInputsCpiWrite,
};
use light_sdk::cpi::v2::CpiAccounts;

use super::CTokenPda;
use crate::ChainedCtokenInstructionData;

pub fn process_mint_action<'a, 'info>(
    ctx: &Context<'_, '_, '_, 'info, CTokenPda<'info>>,
    input: &ChainedCtokenInstructionData,
    cpi_accounts: &CpiAccounts<'a, 'info>,
) -> Result<()> {
    let actions = vec![
        MintActionType::MintTo {
            recipients: input.token_recipients.clone(),
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
        cpi_context: light_ctoken_types::instructions::mint_action::CpiContext {
            set_context: false,
            first_set_context: true,
            in_tree_index: 0,
            in_queue_index: 0,
            out_queue_index: 1,
            token_out_queue_index: 1,
            assigned_account_index: 0,
            ..Default::default()
        },
        cpi_context_pubkey: *cpi_accounts.cpi_context().unwrap().key,
    };

    // Build using the new builder pattern
    let mint_action_inputs2 = MintActionInputsCpiWrite::new_create_mint(CreateMintCpiWriteInputs {
        compressed_mint_inputs: input.compressed_mint_with_context.clone(),
        mint_seed: ctx.accounts.mint_seed.key(),
        mint_bump: input.mint_bump,
        authority: ctx.accounts.mint_authority.key(),
        payer: ctx.accounts.payer.key(),
        cpi_context_pubkey: *cpi_accounts.cpi_context().unwrap().key,
        first_set_context: true,
        address_tree_index: 0,
        output_queue_index: 1,
        assigned_account_index: 0,
    })
    .add_mint_to(
        input.token_recipients.clone(),
        2, // token_account_version
        1, // token_out_queue_index
    )
    .unwrap() // add_mint_to returns Result in CPI write mode
    .add_update_mint_authority(input.final_mint_authority);

    // Assert that the builder produces the same result as manual construction
    assert_eq!(mint_action_inputs, mint_action_inputs2);

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
