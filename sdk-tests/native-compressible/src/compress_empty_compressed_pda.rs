use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    compressible::{compress_pda_native, CompressibleConfig},
    cpi::CpiAccountsSmall,
    error::LightSdkError,
    instruction::{account_meta::CompressedAccountMeta, ValidityProof},
};
use light_sdk_types::CpiAccountsConfig;
use solana_program::{account_info::AccountInfo, msg};

use crate::MyPdaAccount;

/// Generic instruction data for compress empty compressed PDA
/// This compresses a PDA that was created via create_empty_compressed_pda
#[derive(BorshDeserialize, BorshSerialize)]
pub struct CompressEmptyCompressedPdaInstruction {
    pub proof: ValidityProof,
    pub compressed_account_meta: CompressedAccountMeta,
}

/// Compresses a PDA that was created with empty compressed account back into a compressed account
/// This is the second step after create_empty_compressed_pda
pub fn compress_empty_compressed_pda(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    let mut instruction_data = instruction_data;
    let instruction_data =
        CompressEmptyCompressedPdaInstruction::deserialize(&mut instruction_data).map_err(|e| {
            solana_program::msg!(
                "Failed to deserialize CompressEmptyCompressedPdaInstruction: {:?}",
                e
            );
            LightSdkError::Borsh
        })?;

    let solana_account = &mut accounts[1].clone();
    let config_account = &accounts[2];
    let rent_recipient = &accounts[3];

    // Load config
    let config = CompressibleConfig::load_checked(config_account, &crate::ID)?;

    // CHECK: rent recipient from config
    if rent_recipient.key != &config.rent_recipient {
        solana_program::msg!(
            "Rent recipient does not match config: {:?} != {:?}",
            rent_recipient.key,
            config.rent_recipient
        );
        return Err(LightSdkError::ConstraintViolation);
    }

    // Cpi accounts
    let cpi_config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
    let cpi_accounts = CpiAccountsSmall::new_with_config(&accounts[0], &accounts[4..], cpi_config);

    // Deserialize the PDA account data (skip the 8-byte discriminator)
    // Use a scope to ensure the borrow is dropped before compression
    let mut pda_data = {
        let account_data = solana_account.data.borrow();
        msg!("pda account: {:?}", account_data);

        MyPdaAccount::deserialize(&mut &account_data[8..]).map_err(|e| {
            solana_program::msg!("Failed to deserialize MyPdaAccount: {:?}", e);
            LightSdkError::Borsh
        })?
    }; // account_data borrow is dropped here

    msg!("Compressing PDA that was created with empty compressed account");

    compress_pda_native::<MyPdaAccount>(
        solana_account,
        &mut pda_data,
        &instruction_data.compressed_account_meta,
        instruction_data.proof,
        cpi_accounts,
        rent_recipient,
        &config.compression_delay,
    )?;

    Ok(())
}
