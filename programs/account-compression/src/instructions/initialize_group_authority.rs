use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use crate::{config_accounts::GroupAuthority, utils::constants::GROUP_AUTHORITY_SEED};

#[derive(Accounts)]
#[instruction(seed: [u8; 32])]
pub struct InitializeGroupAuthority<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        seeds = [GROUP_AUTHORITY_SEED, seed.as_slice()],
        bump,
        space = GroupAuthority::LEN,
    )]
    pub group_authority: Account<'info, GroupAuthority>,
    pub system_program: Program<'info, System>,
}
