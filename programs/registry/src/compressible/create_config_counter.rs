use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;

pub const COMPRESSIBLE_CONFIG_COUNTER_SEED: &[u8] = b"compressible_config_counter";

/// Account that tracks the number of compressible configs created
#[aligned_sized(anchor)]
#[account]
#[derive(Debug)]
pub struct ConfigCounter {
    /// The counter value tracking number of configs
    pub counter: u16,
}

/// Context for creating the config counter PDA
#[derive(Accounts)]
pub struct CreateConfigCounter<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// Authority from the protocol config - must be signer
    pub authority: Signer<'info>,

    /// CHECK: authority is protocol config authority.
    #[account(has_one = authority)]
    pub protocol_config_pda: Account<'info, crate::protocol_config::state::ProtocolConfigPda>,

    #[account(
        init,
        seeds = [COMPRESSIBLE_CONFIG_COUNTER_SEED],
        bump,
        space = ConfigCounter::LEN,
        payer = fee_payer
    )]
    pub config_counter: Account<'info, ConfigCounter>,

    pub system_program: Program<'info, System>,
}
