use account_compression::{program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED};
use anchor_lang::prelude::*;

use crate::protocol_config::state::ProtocolConfigPda;

#[derive(Accounts)]
pub struct InitializeBatchedAddressTree<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK:  initialized in account compression program.
    #[account(zero)]
    pub merkle_tree: AccountInfo<'info>,
    /// CHECK: (account compression program) access control.
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK: (seed constraints) used to invoke account compression program via cpi.
    #[account(mut, seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority: AccountInfo<'info>,
    pub account_compression_program: Program<'info, AccountCompression>,
    pub protocol_config_pda: Account<'info, ProtocolConfigPda>,
}

pub fn process_initialize_batched_address_merkle_tree(
    ctx: &Context<InitializeBatchedAddressTree>,
    bump: u8,
    params: Vec<u8>,
) -> Result<()> {
    let bump = &[bump];
    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];
    let accounts = account_compression::cpi::accounts::InitializeBatchedAddressMerkleTree {
        authority: ctx.accounts.cpi_authority.to_account_info(),
        merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
        registered_program_pda: Some(ctx.accounts.registered_program_pda.clone()),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.account_compression_program.to_account_info(),
        accounts,
        signer_seeds,
    );

    account_compression::cpi::initialize_batched_address_merkle_tree(cpi_ctx, params)
}
