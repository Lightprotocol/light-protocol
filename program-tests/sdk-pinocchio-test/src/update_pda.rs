use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk_pinocchio::{
    account::LightAccount,
    cpi::{CpiAccounts, CpiAccountsConfig, CpiInputs},
    error::LightSdkError,
    instruction::account_meta::CompressedAccountMeta,
    ValidityProof,
};
use pinocchio::account_info::AccountInfo;

use crate::create_pda::MyCompressedAccount;

/// CU usage:
/// - sdk pre system program  9,183k CU
/// - total with V2 tree: 50,194 CU (proof by index)
pub fn update_pda<const BATCHED: bool>(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    let mut instruction_data = instruction_data;
    let instruction_data = UpdatePdaInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;

    let program_id = crate::ID.into();
    let mut my_compressed_account = LightAccount::<'_, MyCompressedAccount>::new_mut(
        &program_id,
        &instruction_data.my_compressed_account.meta,
        MyCompressedAccount {
            data: instruction_data.my_compressed_account.data,
        },
    )?;

    my_compressed_account.data = instruction_data.new_data;

    let config = CpiAccountsConfig {
        self_program: crate::ID,
        cpi_context: false,
        sol_pool_pda: false,
        sol_compression_recipient: false,
    };
    let cpi_accounts = CpiAccounts::new_with_config(
        &accounts[0],
        &accounts[instruction_data.system_accounts_offset as usize..],
        config,
    )?;
    let cpi_inputs = CpiInputs::new(
        instruction_data.proof,
        vec![my_compressed_account.to_account_info()?],
    );
    cpi_inputs.invoke_light_system_program(cpi_accounts)?;

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
