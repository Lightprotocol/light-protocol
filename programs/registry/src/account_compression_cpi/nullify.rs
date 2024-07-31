use account_compression::{
    program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED, RegisteredProgram,
};
use anchor_lang::prelude::*;

use crate::epoch::register_epoch::ForesterEpochPda;

#[derive(Accounts)]
pub struct NullifyLeaves<'info> {
    /// CHECK:
    #[account(mut)]
    pub registered_forester_pda: Account<'info, ForesterEpochPda>,
    /// CHECK: unchecked for now logic that regulates forester access is yet to be added.
    pub authority: Signer<'info>,
    /// CHECK:
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority: AccountInfo<'info>,
    /// CHECK:
    #[account(
        seeds = [&crate::ID.to_bytes()], bump, seeds::program = &account_compression::ID,
        )]
    pub registered_program_pda: Account<'info, RegisteredProgram>,
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK: when emitting event.
    pub log_wrapper: UncheckedAccount<'info>,
    /// CHECK: in account compression program
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
    /// CHECK: in account compression program
    #[account(mut)]
    pub nullifier_queue: AccountInfo<'info>,
}

pub fn process_nullify(
    ctx: Context<NullifyLeaves>,
    bump: u8,
    change_log_indices: Vec<u64>,
    leaves_queue_indices: Vec<u16>,
    indices: Vec<u64>,
    proofs: Vec<Vec<[u8; 32]>>,
) -> Result<()> {
    let bump = &[bump];
    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];
    let accounts = account_compression::cpi::accounts::NullifyLeaves {
        authority: ctx.accounts.cpi_authority.to_account_info(),
        registered_program_pda: Some(ctx.accounts.registered_program_pda.to_account_info()),
        log_wrapper: ctx.accounts.log_wrapper.to_account_info(),
        merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
        nullifier_queue: ctx.accounts.nullifier_queue.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.account_compression_program.to_account_info(),
        accounts,
        signer_seeds,
    );

    account_compression::cpi::nullify_leaves(
        cpi_ctx,
        change_log_indices,
        leaves_queue_indices,
        indices,
        proofs,
    )
}
