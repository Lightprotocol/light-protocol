use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    account::LightAccount,
    cpi::{
        v1::CpiAccounts, v2::LightSystemProgramCpi, CpiAccountsConfig, InvokeLightSystemProgram,
        LightCpiInstruction,
    },
    error::LightSdkError,
    instruction::{account_meta::CompressedAccountMeta, ValidityProof},
};
use solana_program::account_info::AccountInfo;

use crate::{create_pda::MyCompressedAccount, ARRAY_LEN};

/// CU usage:
/// - sdk pre system program  9,183k CU
/// - total with V2 tree: 49,044 CU (proof by index)
pub fn update_pda<const BATCHED: bool>(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    let mut instruction_data = instruction_data;
    let instruction_data = UpdatePdaInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;

    let mut my_compressed_account = LightAccount::<MyCompressedAccount>::new_mut(
        &crate::ID,
        &instruction_data.my_compressed_account.meta,
        MyCompressedAccount {
            data: instruction_data.my_compressed_account.data,
        },
    )?;

    my_compressed_account.data = instruction_data.new_data;

    let config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);

    let cpi_accounts = CpiAccounts::try_new_with_config(
        &accounts[0],
        &accounts[instruction_data.system_accounts_offset as usize..],
        config,
    )?;

    LightSystemProgramCpi::new_cpi(crate::LIGHT_CPI_SIGNER, instruction_data.proof)
        .mode_v1()
        .with_light_account(my_compressed_account)?
        .invoke(cpi_accounts)?;
    Ok(())
}

#[derive(Clone, Debug, BorshDeserialize, BorshSerialize)]
pub struct UpdatePdaInstructionData {
    pub proof: ValidityProof,
    pub my_compressed_account: UpdateMyCompressedAccount,
    pub new_data: [u8; ARRAY_LEN],
    pub system_accounts_offset: u8,
}
impl Default for UpdatePdaInstructionData {
    fn default() -> Self {
        Self {
            new_data: [0u8; ARRAY_LEN],
            my_compressed_account: UpdateMyCompressedAccount::default(),
            system_accounts_offset: 0,
            proof: ValidityProof::default(),
        }
    }
}

#[derive(Clone, Debug, BorshDeserialize, BorshSerialize)]
pub struct UpdateMyCompressedAccount {
    pub meta: CompressedAccountMeta,
    pub data: [u8; ARRAY_LEN],
}

impl Default for UpdateMyCompressedAccount {
    fn default() -> Self {
        Self {
            meta: CompressedAccountMeta::default(),
            data: [0u8; ARRAY_LEN],
        }
    }
}
