use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_sdk::cpi::v2::CpiAccounts;
use light_token_interface::instructions::mint_action::{
    MintActionCompressedInstructionData, MintToCompressedAction, UpdateAuthority,
};
use light_token_sdk::compressed_token::{
    ctoken_instruction::CTokenInstruction, mint_action::MintActionCpiWriteAccounts,
};

use super::CTokenPda;
use crate::ChainedCtokenInstructionData;

pub fn process_mint_action<'a, 'info>(
    ctx: &Context<'_, '_, '_, 'info, CTokenPda<'info>>,
    input: &ChainedCtokenInstructionData,
    cpi_accounts: &CpiAccounts<'a, 'info>,
) -> Result<()> {
    // Build instruction data using builder pattern
    let mut instruction_data = MintActionCompressedInstructionData::new_mint(
        input.compressed_mint_with_context.root_index,
        light_compressed_account::instruction_data::compressed_proof::CompressedProof::default(), // Dummy proof for CPI write
        input.compressed_mint_with_context.mint.clone().unwrap(),
    );

    // Add MintToCompressed action
    instruction_data = instruction_data.with_mint_to_compressed(MintToCompressedAction {
        token_account_version: 2,
        recipients: input.token_recipients.clone(),
    });

    // Add UpdateMintAuthority action
    instruction_data = instruction_data.with_update_mint_authority(UpdateAuthority {
        new_authority: input
            .final_mint_authority
            .map(|auth| auth.to_bytes().into()),
    });

    instruction_data = instruction_data.with_cpi_context(
        light_token_interface::instructions::mint_action::CpiContext {
            set_context: false,
            first_set_context: true,
            in_tree_index: 0,
            in_queue_index: 0,
            out_queue_index: 1,
            token_out_queue_index: 1,
            assigned_account_index: 0,
            ..Default::default()
        },
    );

    // Build account structure for CPI write
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

    // Build instruction using trait method for CPI write (first set context)
    let mint_action_instruction = instruction_data
        .instruction_write_to_cpi_context_first(&mint_action_account_infos)
        .unwrap();

    invoke(
        &mint_action_instruction,
        &mint_action_account_infos.to_account_infos(),
    )?;

    Ok(())
}
