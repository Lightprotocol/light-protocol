use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_compressed_token_sdk::instructions::{
    create_mint_action_cpi, CreateMintInputs, MintActionInputs,
};
use light_sdk_types::cpi_accounts::v2::CpiAccounts;

use super::{processor::ChainedCtokenInstructionData, PdaCToken};

pub fn process_mint_action<'a, 'info>(
    ctx: &Context<'_, '_, '_, 'info, PdaCToken<'info>>,
    input: &ChainedCtokenInstructionData,
    cpi_accounts: &CpiAccounts<'a, AccountInfo<'info>>,
) -> Result<()> {
    // Derive the output queue pubkey - use the same tree as the PDA creation
    let address_tree_pubkey = *cpi_accounts.tree_accounts().unwrap()[0].key; // Same tree as PDA
    let output_queue = *cpi_accounts.tree_accounts().unwrap()[1].key; // Same tree as PDA

    // Build using the new builder pattern
    let mint_action_inputs = MintActionInputs::new_create_mint(CreateMintInputs {
        compressed_mint_inputs: input.compressed_mint_with_context.clone(),
        mint_seed: ctx.accounts.mint_seed.key(),
        mint_bump: input.mint_bump,
        authority: ctx.accounts.mint_authority.key(),
        payer: ctx.accounts.payer.key(),
        proof: input.pda_creation.proof.into(),
        address_tree: address_tree_pubkey,
        output_queue,
    })
    .add_mint_to(
        input.token_recipients.clone(),
        2, // token_account_version
        Some(output_queue),
    )
    .add_mint_to_decompressed(
        ctx.accounts.token_account.key(),
        input.token_recipients[0].amount,
    )
    .add_update_mint_authority(input.final_mint_authority);

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
            ..Default::default()
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
