use account_compression::{program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED};
use anchor_lang::prelude::*;

use crate::ForesterEpochPda;

#[derive(Accounts)]
pub struct BatchUpdateAddressTree<'info> {
    /// CHECK: only eligible foresters can nullify leaves. Is checked in ix.
    #[account(mut)]
    pub registered_forester_pda: Option<Account<'info, ForesterEpochPda>>,
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
    pub merkle_tree: AccountInfo<'info>,
}

pub fn process_batch_update_address_tree(
    ctx: &Context<BatchUpdateAddressTree>,
    bump: u8,
    data: Vec<u8>,
) -> Result<()> {
    let bump = &[bump];
    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];
    let accounts = account_compression::cpi::accounts::BatchUpdateAddressTree {
        authority: ctx.accounts.cpi_authority.to_account_info(),
        merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
        registered_program_pda: Some(ctx.accounts.registered_program_pda.clone()),
        log_wrapper: ctx.accounts.log_wrapper.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.account_compression_program.to_account_info(),
        accounts,
        signer_seeds,
    );

    account_compression::cpi::batch_update_address_tree(cpi_ctx, data)
}
