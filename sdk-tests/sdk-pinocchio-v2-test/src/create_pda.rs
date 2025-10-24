use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk_pinocchio::{
    cpi::{
        v1::CpiAccountsConfig,
        v2::{CpiAccounts, LightSystemProgramCpi},
        InvokeLightSystemProgram, LightCpiInstruction,
    },
    error::LightSdkError,
    instruction::{PackedAddressTreeInfo, ValidityProof},
    LightAccount, LightDiscriminator, LightHasher,
};
use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

/// CU usage:
/// - sdk pre system program cpi 10,942 CU
/// - total with V1 tree: 307,784 CU
/// - total with V2 tree: 138,876 CU
pub fn create_pda(accounts: &[AccountInfo], instruction_data: &[u8]) -> Result<(), ProgramError> {
    let mut instruction_data = instruction_data;
    let instruction_data = CreatePdaInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| ProgramError::BorshIoError)?;
    let config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
    let cpi_accounts = CpiAccounts::new_with_config(
        &accounts[0],
        &accounts[instruction_data.system_accounts_offset as usize..],
        config,
    );

    let address_tree_info = instruction_data.address_tree_info;

    // Manually get tree pubkey from v2 accounts
    let tree_pubkey = cpi_accounts
        .get_tree_account_info(
            instruction_data
                .address_tree_info
                .address_merkle_tree_pubkey_index as usize,
        )
        .map_err(LightSdkError::from)
        .map_err(ProgramError::from)?
        .key();
    let (address, address_seed) = light_sdk_pinocchio::address::v2::derive_address(
        &[b"compressed", instruction_data.data.as_slice()],
        tree_pubkey,
        &crate::ID,
    );

    let new_address_params =
        address_tree_info.into_new_address_params_assigned_packed(address_seed, Some(0));

    let mut my_compressed_account = LightAccount::<MyCompressedAccount>::new_init(
        &crate::LIGHT_CPI_SIGNER.program_id,
        Some(address),
        instruction_data.output_merkle_tree_index,
    );

    my_compressed_account.data = instruction_data.data;

    LightSystemProgramCpi::new_cpi(crate::LIGHT_CPI_SIGNER, instruction_data.proof)
        .with_light_account(my_compressed_account)?
        .with_new_addresses(&[new_address_params])
        .invoke(cpi_accounts)
}

#[derive(
    Clone, Debug, Default, LightHasher, LightDiscriminator, BorshDeserialize, BorshSerialize,
)]
pub struct MyCompressedAccount {
    pub data: [u8; 31],
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct CreatePdaInstructionData {
    pub proof: ValidityProof,
    pub address_tree_info: PackedAddressTreeInfo,
    pub output_merkle_tree_index: u8,
    pub data: [u8; 31],
    pub system_accounts_offset: u8,
    pub tree_accounts_offset: u8,
}
