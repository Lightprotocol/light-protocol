use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{compressible::process_update_compression_config, error::LightSdkError};
use solana_program::{account_info::AccountInfo, pubkey::Pubkey};

/// Updates an existing compressible config
pub fn process_update_config(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    let mut instruction_data = instruction_data;
    let instruction_data = UpdateConfigInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;

    // Get accounts
    let config_account = &accounts[0];
    let authority = &accounts[1];

    process_update_compression_config(
        config_account,
        authority,
        instruction_data.new_update_authority.as_ref(),
        instruction_data.new_rent_recipient.as_ref(),
        instruction_data.new_address_space,
        instruction_data.new_compression_delay,
        &crate::ID,
    )?;

    Ok(())
}

#[derive(Clone, Debug, BorshDeserialize, BorshSerialize)]
pub struct UpdateConfigInstructionData {
    pub new_update_authority: Option<Pubkey>,
    pub new_rent_recipient: Option<Pubkey>,
    pub new_address_space: Option<Vec<Pubkey>>,
    pub new_compression_delay: Option<u32>,
}
