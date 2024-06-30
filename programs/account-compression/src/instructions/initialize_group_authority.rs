use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use crate::{state::GroupAuthority, utils::constants::GROUP_AUTHORITY_SEED};

#[derive(Accounts)]
pub struct InitializeGroupAuthority<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// Seed public key used to derive the group authority.
    pub seed: Signer<'info>,
    #[account(
        init,
        payer = authority,
        seeds = [GROUP_AUTHORITY_SEED, seed.key().to_bytes().as_slice()],
        bump,
        space = GroupAuthority::LEN,
    )]
    pub group_authority: Account<'info, GroupAuthority>,
    pub system_program: Program<'info, System>,
}
