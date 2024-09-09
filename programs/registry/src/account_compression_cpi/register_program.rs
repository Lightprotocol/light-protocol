use account_compression::{
    program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED, GroupAuthority,
};
use anchor_lang::prelude::*;

use crate::protocol_config::state::ProtocolConfigPda;

#[derive(Accounts)]
pub struct RegisterProgram<'info> {
    /// CHECK: only the protocol authority can register new programs.
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut, has_one = authority)]
    pub protocol_config_pda: Account<'info, ProtocolConfigPda>,
    /// CHECK: (seed constraints) used to invoke account compression program via cpi.
    #[account(mut, seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority: AccountInfo<'info>,
    /// CHECK: (account compression program).
    #[account(mut ,constraint = group_pda.authority == cpi_authority.key())]
    pub group_pda: Account<'info, GroupAuthority>,
    pub account_compression_program: Program<'info, AccountCompression>,
    pub system_program: Program<'info, System>,
    /// CHECK: (account compression program).
    #[account(mut)]
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK: (account compression program). TODO: check that a signer is the upgrade authority.
    /// - is signer so that only the program deployer can register a program.
    pub program_to_be_registered: Signer<'info>,
}

#[derive(Accounts)]
pub struct DeregisterProgram<'info> {
    /// CHECK: only the protocol authority can register new programs.
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut, has_one = authority)]
    pub protocol_config_pda: Account<'info, ProtocolConfigPda>,
    /// CHECK: (seed constraints) used to invoke account compression program via cpi.
    #[account(mut, seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority: AccountInfo<'info>,
    /// CHECK: (account compression program).
    #[account(mut ,constraint = group_pda.authority == cpi_authority.key())]
    pub group_pda: Account<'info, GroupAuthority>,
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK: (account compression program).
    #[account(mut)]
    pub registered_program_pda: AccountInfo<'info>,
}
