use account_compression::{program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED};
use anchor_lang::prelude::*;
use light_merkle_tree_metadata::utils::if_equals_zero_u64;

use crate::{protocol_config::state::ProtocolConfigPda, ForesterEpochPda};

#[derive(Accounts)]
pub struct RolloverBatchedStateMerkleTree<'info> {
    /// CHECK: only eligible foresters can nullify leaves. Is checked in ix.
    #[account(mut)]
    pub registered_forester_pda: Option<Account<'info, ForesterEpochPda>>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK:  initialized in account compression program.
    #[account(mut)]
    pub new_state_merkle_tree: AccountInfo<'info>,
    /// CHECK:  in account compression program.
    #[account(mut)]
    pub old_state_merkle_tree: AccountInfo<'info>,
    /// CHECK:  initialized in account compression program.
    #[account(mut)]
    pub new_output_queue: AccountInfo<'info>,
    /// CHECK:  in account compression program.
    #[account(mut)]
    pub old_output_queue: AccountInfo<'info>,
    /// CHECK: (system program) new cpi context account.
    #[account(mut)]
    pub cpi_context_account: AccountInfo<'info>,
    /// CHECK: (account compression program) access control.
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK: (seed constraints) used to invoke account compression program via cpi.
    #[account(mut, seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority: AccountInfo<'info>,
    pub account_compression_program: Program<'info, AccountCompression>,
    pub protocol_config_pda: Account<'info, ProtocolConfigPda>,
    pub light_system_program: Program<'info, light_system_program::program::LightSystemProgram>,
}

pub fn process_rollover_batched_state_merkle_tree(
    ctx: &Context<RolloverBatchedStateMerkleTree>,
    bump: u8,
) -> Result<()> {
    let bump = &[bump];
    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];
    let accounts = account_compression::cpi::accounts::RolloverBatchedStateMerkleTree {
        fee_payer: ctx.accounts.authority.to_account_info(),
        authority: ctx.accounts.cpi_authority.to_account_info(),
        old_state_merkle_tree: ctx.accounts.old_state_merkle_tree.to_account_info(),
        new_state_merkle_tree: ctx.accounts.new_state_merkle_tree.to_account_info(),
        old_output_queue: ctx.accounts.old_output_queue.to_account_info(),
        new_output_queue: ctx.accounts.new_output_queue.to_account_info(),
        registered_program_pda: Some(ctx.accounts.registered_program_pda.clone()),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.account_compression_program.to_account_info(),
        accounts,
        signer_seeds,
    );
    let network_fee = if ctx.accounts.registered_forester_pda.is_some() {
        if_equals_zero_u64(ctx.accounts.protocol_config_pda.config.network_fee)
    } else {
        None
    };

    account_compression::cpi::rollover_batched_state_merkle_tree(
        cpi_ctx,
        ctx.accounts.protocol_config_pda.config.cpi_context_size,
        network_fee,
    )
}
