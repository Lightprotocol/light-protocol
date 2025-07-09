use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    compressible::{update_config, CompressibleConfig},
    error::LightSdkError,
};
use solana_program::account_info::AccountInfo;
use solana_program::pubkey::Pubkey;

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

    // Verify the config PDA
    let (expected_pda, _) = CompressibleConfig::derive_pda(&crate::ID);
    if config_account.key != &expected_pda {
        return Err(LightSdkError::ConstraintViolation);
    }

    // Update the config
    update_config(
        config_account,
        authority,
        instruction_data.new_update_authority.as_ref(),
        instruction_data.new_rent_recipient.as_ref(),
        instruction_data.new_address_space.as_ref(),
        instruction_data.new_compression_delay,
    )?;

    Ok(())
}

#[derive(Clone, Debug, BorshDeserialize, BorshSerialize)]
pub struct UpdateConfigInstructionData {
    pub new_update_authority: Option<Pubkey>,
    pub new_rent_recipient: Option<Pubkey>,
    pub new_address_space: Option<Pubkey>,
    pub new_compression_delay: Option<u32>,
}
