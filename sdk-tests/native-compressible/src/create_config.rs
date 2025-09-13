use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    compressible::process_initialize_compression_config_checked as sdk_process_initialize_compression_config_checked,
    error::LightSdkError,
};
use solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey};

/// Creates a new compressible config PDA
pub fn process_initialize_compression_config_checked(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    let mut instruction_data = instruction_data;
    msg!("instruction_data: {:?}", instruction_data.len());
    let instruction_data = InitializeCompressionConfigData::deserialize(&mut instruction_data)
        .map_err(|err| {
            msg!(
                "InitializeCompressionConfigData::deserialize error: {:?}",
                err
            );
            LightSdkError::Borsh
        })?;

    // Get accounts
    let payer = &accounts[0];
    let config_account = &accounts[1];
    let program_data_account = &accounts[2];
    let update_authority = &accounts[3];
    let system_program = &accounts[4];

    sdk_process_initialize_compression_config_checked(
        config_account,
        update_authority,
        program_data_account,
        &instruction_data.rent_recipient,
        instruction_data.address_space,
        instruction_data.compression_delay,
        0, // one global config for now, so bump is 0.
        payer,
        system_program,
        &crate::ID,
    )?;

    Ok(())
}

/// Generic instruction data for initialize config
/// Note: Real programs should use their specific instruction format
#[derive(BorshDeserialize, BorshSerialize)]
pub struct InitializeCompressionConfigData {
    pub compression_delay: u32,
    pub rent_recipient: Pubkey,
    pub address_space: Vec<Pubkey>,
}

// Type alias for backward compatibility with tests
pub type CreateConfigInstructionData = InitializeCompressionConfigData;

/// Generic instruction data for update config
/// Note: Real programs should use their specific instruction format  
#[derive(BorshDeserialize, BorshSerialize)]
pub struct UpdateCompressionConfigData {
    pub new_compression_delay: Option<u32>,
    pub new_rent_recipient: Option<Pubkey>,
    pub new_address_space: Option<Vec<Pubkey>>,
    pub new_update_authority: Option<Pubkey>,
}
