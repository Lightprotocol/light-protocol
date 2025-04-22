use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};

use crate::{
    errors::AccountCompressionErrorCode, utils::constants::CPI_AUTHORITY_PDA_SEED, GroupAuthority,
};

#[repr(C)]
#[derive(Debug, Pod, Zeroable, Copy, PartialEq)]
#[account]
#[aligned_sized(anchor)]
pub struct RegisteredProgram {
    pub registered_program_id: Pubkey,
    pub group_authority_pda: Pubkey,
    pub registered_program_signer_pda: Pubkey,
}

#[derive(Accounts)]
pub struct RegisterProgramToGroup<'info> {
    /// CHECK: Signer is checked according to authority pda in instruction.
    #[account( mut, constraint= authority.key() == group_authority_pda.authority @AccountCompressionErrorCode::InvalidAuthority)]
    pub authority: Signer<'info>,
    /// CHECK:
    pub program_to_be_registered: Signer<'info>,
    #[account(
        init,
        payer = authority,
        seeds = [&program_to_be_registered.key().to_bytes()],
        bump,
        space = RegisteredProgram::LEN,
    )]
    pub registered_program_pda: Account<'info, RegisteredProgram>,
    pub group_authority_pda: Account<'info, GroupAuthority>,
    pub system_program: Program<'info, System>,
}

pub fn process_register_program(ctx: Context<RegisterProgramToGroup>) -> Result<()> {
    ctx.accounts.registered_program_pda.registered_program_id =
        ctx.accounts.program_to_be_registered.key();
    ctx.accounts.registered_program_pda.group_authority_pda =
        ctx.accounts.group_authority_pda.key();
    ctx.accounts
        .registered_program_pda
        .registered_program_signer_pda = Pubkey::find_program_address(
        &[CPI_AUTHORITY_PDA_SEED],
        &ctx.accounts.program_to_be_registered.key(),
    )
    .0;
    Ok(())
}
