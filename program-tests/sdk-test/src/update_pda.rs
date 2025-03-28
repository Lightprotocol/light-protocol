use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    account::CBorshAccount,
    cpi::{
        accounts::{CompressionCpiAccounts, CompressionCpiAccountsConfig},
        verify::verify_compressed_account_infos,
    },
    error::LightSdkError,
    instruction::{account_meta::CompressedAccountMeta, instruction_data::LightInstructionData},
};
use solana_program::account_info::AccountInfo;

use crate::create_pda::MyCompressedAccount;

/// CU usage:
/// - sdk pre system program  10,902k CU
/// - total with V2 tree: 78,074 CU (proof by index)
pub fn update_pda<const BATCHED: bool>(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    let mut instruction_data = instruction_data;
    let instruction_data = UpdatePdaInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;

    let program_id = crate::ID.into();
    let mut my_compressed_account = CBorshAccount::<'_, MyCompressedAccount>::new_mut(
        &program_id,
        &instruction_data.my_compressed_account.meta,
        MyCompressedAccount {
            data: instruction_data.my_compressed_account.data,
        },
    )?;

    my_compressed_account.data = instruction_data.new_data;

    let config = CompressionCpiAccountsConfig {
        self_program: crate::ID,
        cpi_context: false,
        sol_pool_pda: false,
        sol_compression_recipient: false,
    };
    let light_cpi_accounts =
        CompressionCpiAccounts::new_with_config(&accounts[0], &accounts[1..], config)?;

    verify_compressed_account_infos(
        &light_cpi_accounts,
        instruction_data.light_ix_data.proof,
        &[my_compressed_account.to_account_info()?],
        None,
        None,
        false,
        None,
    )
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct UpdatePdaInstructionData {
    pub light_ix_data: LightInstructionData,
    pub my_compressed_account: UpdateMyCompressedAccount,
    pub new_data: [u8; 31],
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct UpdateMyCompressedAccount {
    pub meta: CompressedAccountMeta,
    pub data: [u8; 31],
}
