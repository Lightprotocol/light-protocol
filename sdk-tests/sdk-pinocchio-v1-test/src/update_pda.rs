use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk_pinocchio::{
    cpi::{
        v1::{CpiAccounts, CpiAccountsConfig},
        InvokeLightSystemProgram, LightCpiInstruction,
    },
    error::LightSdkError,
    instruction::{account_meta::CompressedAccountMeta, ValidityProof},
    LightAccount,
};
use pinocchio::{
    account_info::AccountInfo, log::sol_log_compute_units, program_error::ProgramError,
};

use crate::create_pda::MyCompressedAccount;

/// CU usage:
/// - sdk pre system program  9,183k CU
/// - total with V2 tree: 50,194 CU (proof by index)
/// - total with V2 tree: 67,723 CU (proof by index)
pub fn update_pda(accounts: &[AccountInfo], instruction_data: &[u8]) -> Result<(), LightSdkError> {
    sol_log_compute_units();
    let mut instruction_data = instruction_data;
    let instruction_data = UpdatePdaInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;
    sol_log_compute_units();

    let mut my_compressed_account = LightAccount::<MyCompressedAccount>::new_mut(
        &crate::LIGHT_CPI_SIGNER.program_id,
        &instruction_data.my_compressed_account.meta,
        MyCompressedAccount {
            data: instruction_data.my_compressed_account.data,
        },
    )
    .map_err(|e| LightSdkError::ProgramError(ProgramError::Custom(u64::from(e) as u32)))?;
    sol_log_compute_units();

    my_compressed_account.data = instruction_data.new_data;

    let config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
    sol_log_compute_units();
    let cpi_accounts = CpiAccounts::try_new_with_config(
        &accounts[0],
        &accounts[instruction_data.system_accounts_offset as usize..],
        config,
    )?;
    sol_log_compute_units();

    // Use trait-based API
    use light_sdk_pinocchio::cpi::v1::LightSystemProgramCpi;
    let cpi_instruction =
        LightSystemProgramCpi::new_cpi(crate::LIGHT_CPI_SIGNER, instruction_data.proof)
            .with_light_account(my_compressed_account)?;
    sol_log_compute_units();
    cpi_instruction
        .invoke(cpi_accounts)
        .map_err(LightSdkError::from)?;

    Ok(())
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct UpdatePdaInstructionData {
    pub proof: ValidityProof,
    pub my_compressed_account: UpdateMyCompressedAccount,
    pub new_data: [u8; 31],
    pub system_accounts_offset: u8,
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct UpdateMyCompressedAccount {
    pub meta: CompressedAccountMeta,
    pub data: [u8; 31],
}
