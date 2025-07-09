use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    compressible::{create_config, CompressibleConfig},
    error::LightSdkError,
};
use solana_program::account_info::AccountInfo;
use solana_program::pubkey::Pubkey;

/// Creates a new compressible config PDA
pub fn process_create_config(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    let mut instruction_data = instruction_data;
    let instruction_data = CreateConfigInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;

    // Get accounts
    let payer = &accounts[0];
    let config_account = &accounts[1];
    let system_program = &accounts[2];

    // Verify the config PDA
    let (expected_pda, _) = CompressibleConfig::derive_pda(&crate::ID);
    if config_account.key != &expected_pda {
        return Err(LightSdkError::ConstraintViolation);
    }

    // Create the config
    create_config(
        config_account,
        &instruction_data.update_authority,
        &instruction_data.rent_recipient,
        &instruction_data.address_space,
        instruction_data.compression_delay,
        payer,
        system_program,
        &crate::ID,
    )?;

    Ok(())
}

#[derive(Clone, Debug, BorshDeserialize, BorshSerialize)]
pub struct CreateConfigInstructionData {
    pub update_authority: Pubkey,
    pub rent_recipient: Pubkey,
    pub address_space: Pubkey,
    pub compression_delay: u32,
}
