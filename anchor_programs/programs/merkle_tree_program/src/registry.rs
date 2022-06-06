use anchor_lang::prelude::*;
use std::mem::size_of;

#[account]
#[derive(Default)]
pub struct Registry {
    pub bump: u8,
    pub id: Pubkey,
    pub paused: u8,
}

#[derive(Accounts)]
pub struct RegisterNewId<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        seeds = [new_id.key().as_ref()],
        bump,
        space = 8 + size_of::<Registry>(),
        payer = authority,
    )]
    pub registry: Account<'info, Registry>,

    /// CHECK: we don't need to check if it's program because we will pass it as an owner of pda
    pub new_id: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> RegisterNewId<'info> {
    pub fn handle(&mut self, bump: u8) -> Result<()>{
        self.registry.bump = bump;
        self.registry.id = self.new_id.key();
        self.registry.paused = 0;
        Ok(())
    }
} 
