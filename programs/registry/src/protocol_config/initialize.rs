use crate::protocol_config::state::ProtocolConfigPda;
use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

#[constant]
pub const AUTHORITY_PDA_SEED: &[u8] = b"authority";

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct InitializeAuthority<'info> {
    // TODO: add check that this is upgrade authority
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK:
    #[account(init, seeds = [AUTHORITY_PDA_SEED], bump, space = ProtocolConfigPda::LEN, payer = authority)]
    pub authority_pda: Account<'info, ProtocolConfigPda>,
    pub system_program: Program<'info, System>,
    pub mint: Account<'info, Mint>,
    /// CHECK:
    #[account(
        seeds = [CPI_AUTHORITY_PDA_SEED],
        bump,
    )]
    pub cpi_authority: AccountInfo<'info>,
}
