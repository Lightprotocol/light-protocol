use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk_pinocchio::{
    account::LightAccount,
    cpi::{CpiAccounts, CpiAccountsConfig, CpiInputs},
    error::LightSdkError,
    instruction::PackedAddressTreeInfo,
    light_hasher::hash_to_field_size::hashv_to_bn254_field_size_be_const_array,
    LightDiscriminator, LightHasher, ValidityProof,
};
use pinocchio::account_info::AccountInfo;

/// CU usage:
/// - sdk pre system program cpi 10,942 CU
/// - total with V1 tree: 307,784 CU
/// - total with V2 tree: 138,876 CU
pub fn create_pda<const BATCHED: bool>(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    let mut instruction_data = instruction_data;
    let instruction_data = CreatePdaInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;
    let config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
    let cpi_accounts = CpiAccounts::new_with_config(
        &accounts[0],
        &accounts[instruction_data.system_accounts_offset as usize..],
        config,
    )?;

    let address_tree_info = instruction_data.address_tree_info;
    let (address, address_seed) = if BATCHED {
        let tree_acounts = cpi_accounts.tree_accounts();
        let index = tree_acounts[instruction_data
            .address_tree_info
            .address_merkle_tree_pubkey_index as usize]
            .key();
        let address_seed = hashv_to_bn254_field_size_be_const_array::<3>(&[
            b"compressed",
            instruction_data.data.as_slice(),
        ])?;
        let address = light_sdk_pinocchio::light_compressed_account::address::derive_address(
            &address_seed,
            index,
            &crate::ID,
        );
        (address, address_seed)
    } else {
        light_sdk_pinocchio::address::v1::derive_address(
            &[b"compressed", instruction_data.data.as_slice()],
            cpi_accounts.tree_accounts()
                [address_tree_info.address_merkle_tree_pubkey_index as usize]
                .key(),
            &crate::ID,
        )
    };

    let new_address_params = address_tree_info.into_new_address_params_packed(address_seed);

    let mut my_compressed_account = LightAccount::<'_, MyCompressedAccount>::new_init(
        &crate::ID,
        Some(address),
        instruction_data.output_merkle_tree_index,
    );

    my_compressed_account.data = instruction_data.data;

    let cpi_inputs = CpiInputs::new_with_address(
        instruction_data.proof,
        vec![my_compressed_account.to_account_info()?],
        vec![new_address_params],
    );
    cpi_inputs.invoke_light_system_program(cpi_accounts)?;
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
