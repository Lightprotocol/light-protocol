use account_compression::{
    program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED, GroupAuthority,
};
use anchor_lang::prelude::*;

use crate::{protocol_config::state::ProtocolConfigPda, AUTHORITY_PDA_SEED};

#[derive(Accounts)]
pub struct RegisteredProgram<'info> {
    /// CHECK: only the protocol authority can register new programs.
    #[account(mut, constraint = authority.key() == authority_pda.authority)]
    pub authority: Signer<'info>,
    #[account(mut, seeds = [AUTHORITY_PDA_SEED], bump)]
    pub authority_pda: Account<'info, ProtocolConfigPda>,
    /// CHECK: (seed constraints) used to invoke account compression program via cpi.
    #[account(mut, seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority: AccountInfo<'info>,
    /// CHECK: (account compression program).
    #[account(mut)]
    pub group_pda: Account<'info, GroupAuthority>,
    pub account_compression_program: Program<'info, AccountCompression>,
    pub system_program: Program<'info, System>,
    /// CHECK: (account compression program).
    #[account(mut)]
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK: (account compression program).
    pub program_to_be_registered: Signer<'info>,
}
