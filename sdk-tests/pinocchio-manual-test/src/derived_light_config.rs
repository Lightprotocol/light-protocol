//! Config instructions using SDK functions.

use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::{process_initialize_light_config, process_update_light_config};
use light_compressible::rent::RentConfig;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

/// Params order matches SDK's InitializeCompressionConfigAnchorData.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct InitConfigParams {
    pub write_top_up: u32,
    pub rent_sponsor: [u8; 32],
    pub compression_authority: [u8; 32],
    pub rent_config: RentConfig,
    pub address_space: Vec<[u8; 32]>,
}

/// Account order matches SDK's InitializeRentFreeConfig::build().
/// Order: [payer, config, program_data, authority, system_program]
pub fn process_initialize_config(
    accounts: &[AccountInfo],
    data: &[u8],
) -> Result<(), ProgramError> {
    let params = InitConfigParams::try_from_slice(data).map_err(|_| ProgramError::BorshIoError)?;

    if accounts.len() < 5 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let fee_payer = &accounts[0];
    let config = &accounts[1];
    let _program_data = &accounts[2];
    let authority = &accounts[3];
    let system_program = &accounts[4];

    process_initialize_light_config(
        config,
        authority,
        &params.rent_sponsor,
        &params.compression_authority,
        params.rent_config,
        params.write_top_up,
        params.address_space,
        0, // config_bump
        fee_payer,
        system_program,
        &crate::ID,
    )
    .map_err(|e| ProgramError::Custom(u32::from(e)))
}

pub fn process_update_config(accounts: &[AccountInfo], data: &[u8]) -> Result<(), ProgramError> {
    if accounts.len() < 2 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let authority = &accounts[0];
    let config = &accounts[1];

    let remaining = [*config, *authority];
    process_update_light_config(&remaining, data, &crate::ID)
        .map_err(|e| ProgramError::Custom(u32::from(e)))
}
