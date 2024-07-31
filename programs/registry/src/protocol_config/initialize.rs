use crate::protocol_config::state::ProtocolConfigPda;
use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

#[constant]
pub const PROTOCOL_CONFIG_PDA_SEED: &[u8] = b"authority";

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct InitializeProtocolConfig<'info> {
    /// CHECK: initial authority is program keypair.
    /// The authority should be updated to a different keypair after
    /// initialization.
    #[account(mut, constraint= authority.key() == self_program.key())]
    pub authority: Signer<'info>,
    #[account(init, seeds = [PROTOCOL_CONFIG_PDA_SEED], bump, space = ProtocolConfigPda::LEN, payer = authority)]
    pub protocol_config_pda: Account<'info, ProtocolConfigPda>,
    pub system_program: Program<'info, System>,
    pub mint: Account<'info, Mint>,
    /// CHECK: (seed derivation).
    #[account(
        seeds = [CPI_AUTHORITY_PDA_SEED],
        bump,
    )]
    pub cpi_authority: AccountInfo<'info>,
    pub self_program: Program<'info, crate::program::LightRegistry>,
}
