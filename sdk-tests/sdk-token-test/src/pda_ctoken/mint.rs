use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_compressed_token_sdk::instructions::{
    create_mint_action_cpi, mint_action::MintActionType, MintActionInputs,
};
use light_sdk::cpi::CpiAccountsSmall;

use super::{processor::ChainedCtokenInstructionData, PdaCToken};

pub fn process_mint_action<'c, 'info>(
    ctx: &Context<'_, '_, 'c, 'info, PdaCToken<'info>>,
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
        MintActionType::MintToDecompressed {
            account: ctx.accounts.token_account.key(),
            amount: input.token_recipients[0].amount,
        },
    ];

    // Derive the output queue pubkey - use the same tree as the PDA creation
    let address_tree_pubkey = *cpi_accounts.tree_accounts().unwrap()[0].key; // Same tree as PDA
    let output_queue = *cpi_accounts.tree_accounts().unwrap()[1].key; // Same tree as PDA

    let mint_action_inputs = MintActionInputs {
        compressed_mint_inputs: input.compressed_mint_with_context.clone(),
        mint_seed: ctx.accounts.mint_seed.key(),
        create_mint: true,
        mint_bump: Some(input.mint_bump),
        authority: ctx.accounts.mint_authority.key(),
        payer: ctx.accounts.payer.key(),
        proof: input.pda_creation.proof.into(),
        actions,
        address_tree_pubkey, // Use same tree as PDA
        input_queue: None,   // Not needed for create_mint: true
        output_queue,
        tokens_out_queue: Some(output_queue), // For MintTo actions
        token_pool: None,                     // Not needed for compressed mint creation
                                              /*  cpi_context: Some(light_ctoken_types::instructions::mint_action::CpiContext {
                                                  set_context: false,       // Read from CPI context written in PDA creation
                                                  first_set_context: false, // Not the first, we're reading
                                                  in_tree_index: 1,
                                                  in_queue_index: 0,
                                                  out_queue_index: 0,
                                                  token_out_queue_index: 0,
                                                  // Compressed output account order: 0. escrow account 1. mint, 2. token account
                                                  assigned_account_index: 1, // mint
                                              }),*/
    };

    let mint_action_instruction = create_mint_action_cpi(
        mint_action_inputs,
        Some(light_ctoken_types::instructions::mint_action::CpiContext {
            set_context: false,
            first_set_context: false,
            in_tree_index: 1,
            in_queue_index: 0,
            out_queue_index: 0,
            token_out_queue_index: 0,
            assigned_account_index: 1,
        }),
        Some(*cpi_accounts.cpi_context().unwrap().key),
    )
    .unwrap();

    // Get all account infos needed for the mint action
    let mut account_infos = cpi_accounts.to_account_infos();
    account_infos.push(ctx.accounts.ctoken_cpi_authority.to_account_info());
    account_infos.push(ctx.accounts.ctoken_program.to_account_info());
    account_infos.push(ctx.accounts.mint_authority.to_account_info());
    account_infos.push(ctx.accounts.mint_seed.to_account_info());
    account_infos.push(ctx.accounts.payer.to_account_info());
    account_infos.push(ctx.accounts.token_account.to_account_info());
    msg!("mint_action_instruction {:?}", mint_action_instruction);
    msg!(
        "account infos pubkeys {:?}",
        account_infos
            .iter()
            .map(|info| info.key)
            .collect::<Vec<_>>()
    );
    // Invoke the mint action instruction directly
    invoke(&mint_action_instruction, &account_infos)?;

    Ok(())
}
