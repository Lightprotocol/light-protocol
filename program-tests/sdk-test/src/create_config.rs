use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    compressible::{create_config, CompressibleConfig},
    error::LightSdkError,
};
use solana_program::account_info::AccountInfo;
use solana_program::pubkey::Pubkey;

/// Creates a new compressible config PDA
pub fn process_create_compression_config_checked(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    let mut instruction_data = instruction_data;
    let instruction_data = CreateConfigInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;

    // Get accounts
    let payer = &accounts[0];
    let config_account = &accounts[1];
    let update_authority = &accounts[2];
    let system_program = &accounts[3];
    let program_data_account = &accounts[4];

    // Use the SDK's safe create_config function which validates upgrade authority
    create_compression_config_checked(
        config_account,
        update_authority,
        program_data_account,
        &instruction_data.rent_recipient,
        &instruction_data.address_space,
        instruction_data.compression_delay,
        payer,
        system_program,
        &crate::ID,
    )
}

#[derive(Clone, Debug, BorshDeserialize, BorshSerialize)]
pub struct CreateConfigInstructionData {
    pub rent_recipient: Pubkey,
    pub address_space: Pubkey,
    pub compression_delay: u32,
}
