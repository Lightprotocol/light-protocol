use anchor_lang::{prelude::ProgramError, pubkey};
use light_account_checks::{
    checks::{check_discriminator, check_owner},
    AccountIterator,
};
use light_compressible::config::CompressibleConfig;
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;
use spl_pod::{bytemuck, solana_msg::msg};

#[profile]
#[inline(always)]
pub fn parse_config_account(
    config_account: &AccountInfo,
) -> Result<&CompressibleConfig, ProgramError> {
    // Validate config account owner
    check_owner(
        &pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX").to_bytes(),
        config_account,
    )?;
    // Parse config data
    let data = unsafe { config_account.borrow_data_unchecked() };
    check_discriminator::<CompressibleConfig>(data)?;
    let config = bytemuck::pod_from_bytes::<CompressibleConfig>(&data[8..]).map_err(|e| {
        msg!("Failed to deserialize CompressibleConfig: {:?}", e);
        ProgramError::InvalidAccountData
    })?;

    Ok(config)
}

#[profile]
#[inline(always)]
pub fn next_config_account<'info>(
    iter: &mut AccountIterator<'info, AccountInfo>,
) -> Result<&'info CompressibleConfig, ProgramError> {
    let config_account = iter.next_non_mut("compressible config")?;
    let config = parse_config_account(config_account)?;

    // Validate config is active (only active allowed for account creation)
    config.validate_active().map_err(ProgramError::from)?;

    Ok(config)
}
