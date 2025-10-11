use anchor_lang::prelude::*;
use light_compressible::config::CompressibleConfig;

/// Context for updating a compressible config
#[derive(Accounts)]
pub struct UpdateCompressibleConfig<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// Authority that can update the config - must match the config's update_authority
    pub update_authority: Signer<'info>,

    #[account(
        mut,
        has_one = update_authority
    )]
    pub compressible_config: Account<'info, CompressibleConfig>,

    pub system_program: Program<'info, System>,
}
