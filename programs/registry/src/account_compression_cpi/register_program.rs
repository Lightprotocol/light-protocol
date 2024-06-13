use account_compression::{
    program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED, GroupAuthority,
};
use anchor_lang::prelude::*;

use crate::{protocol_config::state::ProtocolConfigPda, AUTHORITY_PDA_SEED};

#[derive(Accounts)]
pub struct RegisteredProgram<'info> {
    #[account(mut, constraint = authority.key() == authority_pda.authority)]
    pub authority: Signer<'info>,
    /// CHECK:
    #[account(mut, seeds = [AUTHORITY_PDA_SEED], bump)]
    pub authority_pda: Account<'info, ProtocolConfigPda>,
    /// CHECK: this is
    #[account(mut, seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority: AccountInfo<'info>,
    #[account(mut)]
    pub group_pda: Account<'info, GroupAuthority>,
    pub account_compression_program: Program<'info, AccountCompression>,
    pub system_program: Program<'info, System>,
    /// CHECK:
    #[account(mut)]
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK: is checked in the account compression program.
    pub program_to_be_registered: Signer<'info>,
}
