//! Config instructions using SDK functions.

use anchor_lang::prelude::*;
use light_compressible::rent::RentConfig;
use light_sdk::interface::program::config::create::process_initialize_light_config;
use solana_program_error::ProgramError;

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
        &params.rent_sponsor.to_bytes(),
        &params.compression_authority.to_bytes(),
        params.rent_config,
        params.write_top_up,
        params.address_space.iter().map(|p| p.to_bytes()).collect(),
        0, // config_bump
        &ctx.accounts.fee_payer,
        &ctx.accounts.system_program,
        &crate::ID.to_bytes(),
    )
    .map_err(|e| anchor_lang::error::Error::from(ProgramError::Custom(u32::from(e))))
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
    instruction_data: Vec<u8>,
) -> Result<()> {
    let remaining = [
        ctx.accounts.config.to_account_info(),
        ctx.accounts.authority.to_account_info(),
    ];
    light_sdk::interface::process_update_light_config(&remaining, &instruction_data, &crate::ID.to_bytes())
        .map_err(|e| anchor_lang::error::Error::from(ProgramError::Custom(u32::from(e))))
}
