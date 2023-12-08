use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use crate::GroupAuthority;

#[derive(Accounts)]
pub struct UpdateGroupAuthority<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
       constraint = group_authority.authority == *authority.key,
    )]
    pub group_authority: Account<'info, GroupAuthority>,
}

pub fn set_group_authority<'info>(
    group_authority: &mut Account<'info, GroupAuthority>,
    authority: Pubkey,
) -> Result<()> {
    group_authority.authority = authority;
    Ok(())
}
