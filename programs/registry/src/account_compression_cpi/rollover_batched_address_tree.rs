use account_compression::{program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED};
use anchor_lang::prelude::*;
use light_merkle_tree_metadata::utils::if_equals_zero_u64;

use crate::{protocol_config::state::ProtocolConfigPda, ForesterEpochPda};

#[derive(Accounts)]
pub struct RolloverBatchedAddressMerkleTree<'info> {
    /// CHECK: only eligible foresters can nullify leaves. Is checked in ix.
    #[account(mut)]
    pub registered_forester_pda: Option<Account<'info, ForesterEpochPda>>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK:  initialized in account compression program.
    #[account(mut)]
    pub new_address_merkle_tree: AccountInfo<'info>,
    /// CHECK:  in account compression program.
    #[account(mut)]
    pub old_address_merkle_tree: AccountInfo<'info>,
    /// CHECK: (account compression program) access control.
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK: (seed constraints) used to invoke account compression program via cpi.
    #[account(mut, seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority: AccountInfo<'info>,
    pub account_compression_program: Program<'info, AccountCompression>,
    pub protocol_config_pda: Account<'info, ProtocolConfigPda>,
}

pub fn process_rollover_batched_address_merkle_tree(
    ctx: &Context<RolloverBatchedAddressMerkleTree>,
    bump: u8,
) -> Result<()> {
    let bump = &[bump];
    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];
    let accounts = account_compression::cpi::accounts::RolloverBatchedAddressMerkleTree {
        fee_payer: ctx.accounts.authority.to_account_info(),
        authority: ctx.accounts.cpi_authority.to_account_info(),
        old_address_merkle_tree: ctx.accounts.old_address_merkle_tree.to_account_info(),
        new_address_merkle_tree: ctx.accounts.new_address_merkle_tree.to_account_info(),
        registered_program_pda: Some(ctx.accounts.registered_program_pda.clone()),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.account_compression_program.to_account_info(),
        accounts,
        signer_seeds,
    );

    account_compression::cpi::rollover_batched_address_merkle_tree(
        cpi_ctx,
        if_equals_zero_u64(ctx.accounts.protocol_config_pda.config.address_network_fee),
    )
}
