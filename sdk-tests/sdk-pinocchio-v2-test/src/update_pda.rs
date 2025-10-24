use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk_pinocchio::{
    cpi::{
        v1::CpiAccountsConfig,
        v2::{CpiAccounts, LightSystemProgramCpi},
        InvokeLightSystemProgram, LightCpiInstruction,
    },
    instruction::{account_meta::CompressedAccountMeta, ValidityProof},
    LightAccount,
};
use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

use crate::{create_pda::MyCompressedAccount, LIGHT_CPI_SIGNER};

/// CU usage:
/// - sdk pre system program  9,183k CU
/// - total with V2 tree: 50,194 CU (proof by index)
/// - total with V2 tree: 67,723 CU (proof by index)
pub fn update_pda<const BATCHED: bool>(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let mut instruction_data = instruction_data;
    let instruction_data = UpdatePdaInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| ProgramError::BorshIoError)?;

    let mut my_compressed_account = LightAccount::<MyCompressedAccount>::new_mut(
        &LIGHT_CPI_SIGNER.program_id,
        &instruction_data.my_compressed_account.meta,
        MyCompressedAccount {
            data: instruction_data.my_compressed_account.data,
        },
    )
    .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;

    my_compressed_account.data = instruction_data.new_data;

    let config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
    let cpi_accounts = CpiAccounts::new_with_config(
        &accounts[0],
        &accounts[instruction_data.system_accounts_offset as usize..],
        config,
    );

    LightSystemProgramCpi::new_cpi(crate::LIGHT_CPI_SIGNER, instruction_data.proof)
        .with_light_account(my_compressed_account)?
        .invoke(cpi_accounts)
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
