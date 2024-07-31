use account_compression::{
    program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED, RegisteredProgram,
};
use anchor_lang::prelude::*;

use crate::epoch::register_epoch::ForesterEpochPda;

#[derive(Accounts)]
pub struct RolloverMerkleTreeAndQueue<'info> {
    /// CHECK:
    #[account(mut)]
    pub registered_forester_pda: Account<'info, ForesterEpochPda>,
    /// CHECK: unchecked for now logic that regulates forester access is yet to be added.
    #[account(mut)]
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
    /// CHECK:
    #[account(zero)]
    pub new_merkle_tree: AccountInfo<'info>,
    /// CHECK:
    #[account(zero)]
    pub new_queue: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    pub old_merkle_tree: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    pub old_queue: AccountInfo<'info>,
}

pub fn process_rollover_address_merkle_tree_and_queue(
    ctx: Context<RolloverMerkleTreeAndQueue>,
    bump: u8,
) -> Result<()> {
    let bump = &[bump];

    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];

    let accounts = account_compression::cpi::accounts::RolloverAddressMerkleTreeAndQueue {
        fee_payer: ctx.accounts.authority.to_account_info(),
        authority: ctx.accounts.cpi_authority.to_account_info(),
        registered_program_pda: Some(ctx.accounts.registered_program_pda.to_account_info()),
        new_address_merkle_tree: ctx.accounts.new_merkle_tree.to_account_info(),
        new_queue: ctx.accounts.new_queue.to_account_info(),
        old_address_merkle_tree: ctx.accounts.old_merkle_tree.to_account_info(),
        old_queue: ctx.accounts.old_queue.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.account_compression_program.to_account_info(),
        accounts,
        signer_seeds,
    );

    account_compression::cpi::rollover_address_merkle_tree_and_queue(cpi_ctx)
}
pub fn process_rollover_state_merkle_tree_and_queue(
    ctx: Context<RolloverMerkleTreeAndQueue>,
    bump: u8,
) -> Result<()> {
    let bump = &[bump];

    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];

    let accounts = account_compression::cpi::accounts::RolloverStateMerkleTreeAndNullifierQueue {
        fee_payer: ctx.accounts.authority.to_account_info(),
        authority: ctx.accounts.cpi_authority.to_account_info(),
        registered_program_pda: Some(ctx.accounts.registered_program_pda.to_account_info()),
        new_state_merkle_tree: ctx.accounts.new_merkle_tree.to_account_info(),
        new_nullifier_queue: ctx.accounts.new_queue.to_account_info(),
        old_state_merkle_tree: ctx.accounts.old_merkle_tree.to_account_info(),
        old_nullifier_queue: ctx.accounts.old_queue.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.account_compression_program.to_account_info(),
        accounts,
        signer_seeds,
    );

    account_compression::cpi::rollover_state_merkle_tree_and_nullifier_queue(cpi_ctx)
}
