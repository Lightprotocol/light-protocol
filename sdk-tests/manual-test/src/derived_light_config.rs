//! Config instructions using SDK functions.

use anchor_lang::prelude::*;
use light_compressible::rent::RentConfig;
use light_sdk::interface::config::{process_initialize_light_config, process_update_light_config};

/// Params order matches SDK's InitializeCompressionConfigAnchorData.
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitConfigParams {
    pub write_top_up: u32,
    pub rent_sponsor: Pubkey,
    pub compression_authority: Pubkey,
    pub rent_config: RentConfig,
    pub address_space: Vec<Pubkey>,
}

/// Account order matches SDK's InitializeRentFreeConfig::build().
/// Order: [payer, config, program_data, authority, system_program]
#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Initialized by SDK function
    #[account(mut)]
    pub config: AccountInfo<'info>,

    /// CHECK: Program data PDA for upgrade authority verification
    pub program_data: AccountInfo<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn process_initialize_config<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeConfig<'info>>,
    params: InitConfigParams,
) -> Result<()> {
    process_initialize_light_config(
        &ctx.accounts.config,
        &ctx.accounts.authority,
        &params.rent_sponsor,
        &params.compression_authority,
        params.rent_config,
        params.write_top_up,
        params.address_space,
        0, // config_bump
        &ctx.accounts.fee_payer,
        &ctx.accounts.system_program,
        &crate::ID,
    )
    .map_err(Into::into)
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct UpdateConfigParams {
    pub new_update_authority: Option<Pubkey>,
    pub new_rent_sponsor: Option<Pubkey>,
    pub new_compression_authority: Option<Pubkey>,
    pub new_rent_config: Option<RentConfig>,
    pub new_write_top_up: Option<u32>,
    pub new_address_space: Option<Vec<Pubkey>>,
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: Validated by SDK function
    #[account(mut)]
    pub config: AccountInfo<'info>,
}

pub fn process_update_config<'info>(
    ctx: Context<'_, '_, '_, 'info, UpdateConfig<'info>>,
    params: UpdateConfigParams,
) -> Result<()> {
    process_update_light_config(
        &ctx.accounts.config,
        &ctx.accounts.authority,
        params.new_update_authority.as_ref(),
        params.new_rent_sponsor.as_ref(),
        params.new_compression_authority.as_ref(),
        params.new_rent_config,
        params.new_write_top_up,
        params.new_address_space,
        &crate::ID,
    )
    .map_err(Into::into)
}
