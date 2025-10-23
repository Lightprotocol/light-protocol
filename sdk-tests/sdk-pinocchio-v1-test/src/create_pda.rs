use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk_pinocchio::{
    cpi::{
        v1::{CpiAccounts, CpiAccountsConfig},
        InvokeLightSystemProgram, LightCpiInstruction,
    },
    error::LightSdkError,
    instruction::{PackedAddressTreeInfo, ValidityProof},
    LightAccount, LightDiscriminator, LightHasher,
};
use pinocchio::account_info::AccountInfo;

/// CU usage:
/// - sdk pre system program cpi 10,942 CU
/// - total with V1 tree: 307,784 CU
/// - total with V2 tree: 138,876 CU
pub fn create_pda(accounts: &[AccountInfo], instruction_data: &[u8]) -> Result<(), LightSdkError> {
    let mut instruction_data = instruction_data;
    let instruction_data = CreatePdaInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;
    let config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
    let cpi_accounts = CpiAccounts::try_new_with_config(
        &accounts[0],
        &accounts[instruction_data.system_accounts_offset as usize..],
        config,
    )?;

    let address_tree_info = instruction_data.address_tree_info;
    let (address, address_seed) = light_sdk_pinocchio::address::v1::derive_address(
        &[b"compressed", instruction_data.data.as_slice()],
        &address_tree_info.get_tree_pubkey(&cpi_accounts)?,
        &crate::ID,
    );

    let new_address_params = address_tree_info.into_new_address_params_packed(address_seed);

    let mut my_compressed_account = LightAccount::<MyCompressedAccount>::new_init(
        &crate::LIGHT_CPI_SIGNER.program_id,
        Some(address),
        instruction_data.output_merkle_tree_index,
    );

    my_compressed_account.data = instruction_data.data;

    // Use trait-based API
    use light_sdk_pinocchio::cpi::v1::LightSystemProgramCpi;
    let cpi_instruction =
        LightSystemProgramCpi::new_cpi(crate::LIGHT_CPI_SIGNER, instruction_data.proof)
            .with_light_account(my_compressed_account)?
            .with_new_addresses(&[new_address_params]);
    cpi_instruction
        .invoke(cpi_accounts)
        .map_err(LightSdkError::from)?;
    Ok(())
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
