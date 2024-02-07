use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;

use crate::{errors::ErrorCode, GroupAuthority};

#[account]
#[aligned_sized(anchor)]
pub struct RegisteredProgram {
    pub pubkey: Pubkey,
}

#[derive(Accounts)]
#[instruction(registered_program_id: Pubkey)]
pub struct RegisterVerifier<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [&registered_program_id.to_bytes()],
        bump,
        space = RegisteredProgram::LEN,
    )]
    pub registered_program_pda: Account<'info, RegisteredProgram>,
    /// CHECK:` Signer is checked according to authority pda in instruction
    #[account(mut, address=group_authority.authority @ErrorCode::InvalidAuthority)]
    pub authority: Signer<'info>,
    pub group_authority: Account<'info, GroupAuthority>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn process_register_program(
    ctx: Context<RegisterVerifier>,
    registered_program_id: Pubkey,
) -> Result<()> {
    ctx.accounts.registered_program_pda.pubkey = registered_program_id;
    Ok(())
}
