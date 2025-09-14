use anchor_lang::prelude::*;
use light_compressible::config::{CompressibleConfig, COMPRESSIBLE_CONFIG_SEED};

/// Context for creating a compressible config
#[derive(Accounts)]
pub struct CreateCompressibleConfig<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// Authority from the protocol config - must be signer
    pub authority: Signer<'info>,

    /// CHECK: authority is protocol config authority.
    #[account(has_one = authority)]
    pub protocol_config_pda: Account<'info, crate::protocol_config::state::ProtocolConfigPda>,

    /// The config counter to increment
    #[account(mut)]
    pub config_counter: Account<'info, super::create_config_counter::ConfigCounter>,

    #[account(
        init,
        seeds = [COMPRESSIBLE_CONFIG_SEED, &config_counter.counter.to_le_bytes()],
        bump,
        space = 8 + std::mem::size_of::<CompressibleConfig>(),
        payer = fee_payer,
    )]
    pub compressible_config: Account<'info, CompressibleConfig>,

    pub system_program: Program<'info, System>,
}
