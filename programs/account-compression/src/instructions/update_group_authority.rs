use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use crate::GroupAuthority;

#[derive(Accounts)]
pub struct UpdateGroupAuthority<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
       constraint = group_authority.authority == *authority.key,
    )]
    pub group_authority: Account<'info, GroupAuthority>,
}

pub fn set_group_authority(
    group_authority: &mut Account<'_, GroupAuthority>,
    authority: Pubkey,
    seed: Option<Pubkey>,
) -> Result<()> {
    group_authority.authority = authority;
    if let Some(seed) = seed {
        group_authority.seed = seed;
    }
    Ok(())
}
