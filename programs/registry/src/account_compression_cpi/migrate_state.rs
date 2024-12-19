use account_compression::{
    program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED, StateMerkleTreeAccount,
};
use anchor_lang::prelude::*;

use crate::epoch::register_epoch::ForesterEpochPda;

#[derive(Accounts)]
pub struct MigrateState<'info> {
    /// CHECK: only eligible foresters can nullify leaves. Is checked in ix.
    #[account(mut)]
    pub registered_forester_pda: Account<'info, ForesterEpochPda>,
    pub authority: Signer<'info>,
    /// CHECK: (seed constraints) used to invoke account compression program via cpi.
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority: AccountInfo<'info>,
    /// CHECK: (account compression program) group access control.
    pub registered_program_pda: AccountInfo<'info>,
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK: (account compression program) when emitting event.
    pub log_wrapper: UncheckedAccount<'info>,
    /// CHECK: (account compression program).
    #[account(mut)]
    pub merkle_tree: AccountLoader<'info, StateMerkleTreeAccount>,
    /// CHECK: (account compression program).
    #[account(mut)]
    pub output_queue: AccountInfo<'info>,
}

pub fn process_migrate_state(
    ctx: &Context<MigrateState>,
    bump: u8,
    input: account_compression::MigrateLeafParams,
) -> Result<()> {
    let bump = &[bump];
    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];
    let accounts = account_compression::cpi::accounts::MigrateState {
        authority: ctx.accounts.cpi_authority.to_account_info(),
        registered_program_pda: Some(ctx.accounts.registered_program_pda.to_account_info()),
        log_wrapper: ctx.accounts.log_wrapper.to_account_info(),
        merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
        output_queue: ctx.accounts.output_queue.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.account_compression_program.to_account_info(),
        accounts,
        signer_seeds,
    );

    account_compression::cpi::migrate_state(cpi_ctx, input)
}
