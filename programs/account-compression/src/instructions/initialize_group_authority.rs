use aligned_sized::aligned_sized;
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

pub const GROUP_AUTHORITY_SEED: &[u8] = b"group_authority";

#[account]
#[aligned_sized(anchor)]
#[derive(Debug)]
pub struct GroupAuthority {
    pub authority: Pubkey,
    pub seed: [u8; 32],
}

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
