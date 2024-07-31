use account_compression::{
    program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED, RegisteredProgram,
};
use anchor_lang::prelude::*;

use crate::epoch::register_epoch::ForesterEpochPda;

#[derive(Accounts)]
pub struct UpdateAddressMerkleTree<'info> {
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
    /// CHECK: (account compression program).
    /// State Merkle tree queue.
    #[account(mut)]
    pub queue: AccountInfo<'info>,
    /// CHECK: (account compression program).
    /// State Merkle tree.
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
    /// CHECK: (account compression program) when emitting event.
    pub log_wrapper: UncheckedAccount<'info>,
}

pub fn process_update_address_merkle_tree(
    ctx: Context<UpdateAddressMerkleTree>,
    bump: u8,
    changelog_index: u16,
    indexed_changelog_index: u16,
    value: u16,
    low_address_index: u64,
    low_address_value: [u8; 32],
    low_address_next_index: u64,
    low_address_next_value: [u8; 32],
    low_address_proof: [[u8; 32]; 16],
) -> Result<()> {
    let bump = &[bump];
    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];

    let accounts = account_compression::cpi::accounts::UpdateAddressMerkleTree {
        authority: ctx.accounts.cpi_authority.to_account_info(),
        registered_program_pda: Some(ctx.accounts.registered_program_pda.to_account_info()),
        log_wrapper: ctx.accounts.log_wrapper.to_account_info(),
        queue: ctx.accounts.queue.to_account_info(),
        merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.account_compression_program.to_account_info(),
        accounts,
        signer_seeds,
    );

    account_compression::cpi::update_address_merkle_tree(
        cpi_ctx,
        changelog_index,
        indexed_changelog_index,
        value,
        low_address_index,
        low_address_value,
        low_address_next_index,
        low_address_next_value,
        low_address_proof,
    )
}
