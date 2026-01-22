use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_compressed_token_sdk::compressed_token::{
    ctoken_instruction::CTokenInstruction, mint_action::MintActionCpiAccounts,
};
use light_sdk_types::cpi_accounts::v2::CpiAccounts;
use light_token_interface::instructions::mint_action::{
    MintActionCompressedInstructionData, MintToAction, MintToCompressedAction, UpdateAuthority,
};

use super::{processor::ChainedCtokenInstructionData, PdaCToken};

pub fn process_mint_action<'a, 'info>(
    ctx: &Context<'_, '_, '_, 'info, PdaCToken<'info>>,
    input: &ChainedCtokenInstructionData,
    cpi_accounts: &CpiAccounts<'a, AccountInfo<'info>>,
) -> Result<()> {
    // Build instruction data using builder pattern
    // ValidityProof is a wrapper around Option<CompressedProof>
    let compressed_proof = input.pda_creation.proof.0.unwrap();
    let instruction_data = MintActionCompressedInstructionData::new_mint(
        input.compressed_mint_with_context.root_index,
        compressed_proof,
        input.compressed_mint_with_context.mint.clone().unwrap(),
    )
    .with_mint_to_compressed(MintToCompressedAction {
        token_account_version: 2,
        recipients: input.token_recipients.clone(),
    })
    .with_mint_to(MintToAction {
        account_index: 0, // Index in remaining accounts
        amount: input.token_recipients[0].amount,
    })
    .with_update_mint_authority(UpdateAuthority {
        new_authority: input
            .final_mint_authority
            .map(|auth| auth.to_bytes().into()),
    })
    .with_cpi_context(
        light_token_interface::instructions::mint_action::CpiContext {
            set_context: false,
            first_set_context: false,
            in_tree_index: 1,
            in_queue_index: 0,
            out_queue_index: 0,
            token_out_queue_index: 0,
            assigned_account_index: 1,
            ..Default::default()
        },
    );

    // Build account structure for CPI - manually construct from CpiAccounts
    let tree_accounts = cpi_accounts.tree_accounts().unwrap();
    let ctoken_accounts_vec = vec![ctx.accounts.token_account.to_account_info()];
    let mint_action_accounts = MintActionCpiAccounts {
        compressed_token_program: ctx.accounts.light_token_program.as_ref(),
        light_system_program: cpi_accounts.system_program().unwrap(),
        mint_signer: Some(ctx.accounts.mint_seed.as_ref()),
        authority: ctx.accounts.mint_authority.as_ref(),
        fee_payer: ctx.accounts.payer.as_ref(),
        compressed_token_cpi_authority: ctx.accounts.light_token_cpi_authority.as_ref(),
        registered_program_pda: cpi_accounts.registered_program_pda().unwrap(),
        account_compression_authority: cpi_accounts.account_compression_authority().unwrap(),
        account_compression_program: cpi_accounts.account_compression_program().unwrap(),
        system_program: cpi_accounts.system_program().unwrap(),
        cpi_context: cpi_accounts.cpi_context().ok(),
        out_output_queue: &tree_accounts[1],       // output queue
        in_merkle_tree: &tree_accounts[0],         // address tree
        in_output_queue: None,                     // Not needed for create
        tokens_out_queue: Some(&tree_accounts[0]), // Same as output queue for mint_to
        ctoken_accounts: &ctoken_accounts_vec,     // For MintToCToken
    };

    // Build instruction using trait method
    let mint_action_instruction = instruction_data.instruction(&mint_action_accounts).unwrap();

    // Get all account infos needed for the mint action
    let mut account_infos = cpi_accounts.to_account_infos();
    account_infos.push(ctx.accounts.light_token_cpi_authority.to_account_info());
    account_infos.push(ctx.accounts.light_token_program.to_account_info());
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
