use account_compression::{
    program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED, GroupAuthority,
};
use anchor_lang::prelude::*;

use crate::{protocol_config::state::ProtocolConfigPda, PROTOCOL_CONFIG_PDA_SEED};

#[derive(Accounts)]
pub struct RegisterSystemProgram<'info> {
    /// CHECK: authority is protocol config authority.
    #[account(mut, constraint = authority.key() == protocol_config_pda.authority)]
    pub authority: Signer<'info>,
    /// CHECK: (seed constraints).
    #[account(seeds = [PROTOCOL_CONFIG_PDA_SEED], bump)]
    pub protocol_config_pda: Account<'info, ProtocolConfigPda>,
    /// CHECK: (seed constraint).
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority: AccountInfo<'info>,
    /// CHECK: (account compression program).
    #[account(mut)]
    pub group_pda: Account<'info, GroupAuthority>,
    pub account_compression_program: Program<'info, AccountCompression>,
    pub system_program: Program<'info, System>,
    /// CHECK: is created by the account compression program.
    #[account(mut)]
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK: (account compression program).
    /// Keypair of the program being registered is signer to prevent a third
    /// party from registering it to a security group.
    pub program_to_be_registered: Signer<'info>,
}
