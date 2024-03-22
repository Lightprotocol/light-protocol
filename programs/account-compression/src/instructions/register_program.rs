use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;

use crate::{errors::AccountCompressionErrorCode, GroupAuthority};

#[derive(Debug)]
#[account]
#[aligned_sized(anchor)]
pub struct RegisteredProgram {
    pub pubkey: Pubkey,
}

#[derive(Accounts)]
#[instruction(verifier_pubkey: Pubkey)]
pub struct RegisterProgramToGroup<'info> {
    /// CHECK:` Signer is checked according to authority pda in instruction
    #[account(mut, address=group_authority_pda.authority @AccountCompressionErrorCode::InvalidAuthority)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        seeds = [&verifier_pubkey.to_bytes()],
        bump,
        space = RegisteredProgram::LEN,
    )]
    pub registered_program_pda: Account<'info, RegisteredProgram>,
    #[account(mut)]
    pub group_authority_pda: Account<'info, GroupAuthority>,
    pub system_program: Program<'info, System>,
}

pub fn process_register_program(
    ctx: Context<RegisterProgramToGroup>,
    verifier_pubkey: Pubkey,
) -> Result<()> {
    ctx.accounts.registered_program_pda.pubkey = verifier_pubkey;
    Ok(())
}
