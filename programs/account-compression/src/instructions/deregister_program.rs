use anchor_lang::prelude::*;

use crate::{errors::AccountCompressionErrorCode, GroupAuthority, RegisteredProgram};

#[derive(Accounts)]
pub struct DeregisterProgram<'info> {
    /// CHECK: Signer is checked according to authority pda in instruction.
    #[account(mut, constraint= authority.key() == group_authority_pda.authority @AccountCompressionErrorCode::InvalidAuthority)]
    pub authority: Signer<'info>,
    #[account(
        mut, close=close_recipient
    )]
    pub registered_program_pda: Account<'info, RegisteredProgram>,
    #[account( constraint= group_authority_pda.key() == registered_program_pda.group_authority_pda @AccountCompressionErrorCode::InvalidGroup)]
    pub group_authority_pda: Account<'info, GroupAuthority>,
    /// CHECK: recipient is not checked.
    #[account(mut)]
    pub close_recipient: AccountInfo<'info>,
}
