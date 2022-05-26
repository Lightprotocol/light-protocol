use anchor_lang::prelude::*;
use std::mem::size_of;
use crate::utils::config::AUTHORITY_SEED;
use crate::program::MerkleTreeProgram;

#[account]
#[derive(Default)]
pub struct AuthorityConfig {
    pub bump: u8,
    pub authority_key: Pubkey,
}

#[derive(Accounts)]
pub struct CreateAuthorityConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        seeds = [AUTHORITY_SEED.as_ref()],
        bump,
        space = 8 + size_of::<AuthorityConfig>(),
        payer = authority,
    )]
    pub authority_config: Account<'info, AuthorityConfig>,

    #[account(
        constraint = merkle_tree_program.programdata_address()? == Some(merkle_tree_program_data.key())
    )]
    pub merkle_tree_program: Program<'info, MerkleTreeProgram>,

    #[account(
        constraint = merkle_tree_program_data.upgrade_authority_address == Some(authority.key()),
    )]
    pub merkle_tree_program_data: Account<'info, ProgramData>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> CreateAuthorityConfig<'info> {
    pub fn handle(&mut self, bump: u8) -> Result<()>{
        self.authority_config.bump = bump;
        self.authority_config.authority_key = self.authority.key();
        Ok(())
    }
}

#[derive(Accounts)]
pub struct UpdateAuthorityConfig<'info> {
    #[account(
        address = authority_config.authority_key
    )]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [AUTHORITY_SEED.as_ref()],
        bump = authority_config.bump,
    )]
    pub authority_config: Account<'info, AuthorityConfig>,
}

impl<'info> UpdateAuthorityConfig<'info> {
    pub fn handle(&mut self, new_authority: Pubkey) -> Result<()>{
        self.authority_config.authority_key = new_authority;
        Ok(())
    }
}
