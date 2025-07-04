use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    address::v1::derive_address,
    cpi::CpiAccounts,
    error::LightSdkError,
    instruction::{PackedAddressTreeInfo, ValidityProof},
};
use light_sdk_types::CpiAccountsConfig;
use solana_program::account_info::AccountInfo;

use crate::{decompress_to_pda::MyPdaAccount, sdk::compress_pda_new::compress_pda_new};

/// Compresses a PDA into a new compressed account
/// This creates a new compressed account with address derived from the PDA address
pub fn compress_from_pda_new(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    let mut instruction_data = instruction_data;
    let instruction_data = CompressFromPdaNewInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;

    let fee_payer = &accounts[0];
    let pda_account = &accounts[1];
    let rent_recipient = &accounts[2]; // can be hardcoded by caller program

    // Cpi accounts
    let cpi_accounts_struct = CpiAccounts::new_with_config(
        fee_payer,
        &accounts[instruction_data.system_accounts_offset as usize..],
        CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER),
    );

    // Get the address tree pubkey
    let address_tree_pubkey = instruction_data
        .address_tree_info
        .get_tree_pubkey(&cpi_accounts_struct)?;

    // TODO: consider ENFORCING on our end that the cPDA is derived from the pda.
    // this would simplify.
    // Can do offchain!
    let (address, address_seed) = derive_address(
        &[pda_account.key.as_ref()],
        &address_tree_pubkey,
        &crate::ID,
    );

    // Can do offchain!
    let new_address_params = instruction_data
        .address_tree_info
        .into_new_address_params_packed(address_seed);

    // Compress the PDA
    compress_pda_new::<MyPdaAccount>(
        pda_account,
        address,
        new_address_params,
        instruction_data.output_state_tree_index,
        instruction_data.proof,
        cpi_accounts_struct,
        &crate::ID,
        rent_recipient,
    )?;

    Ok(())
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct CompressFromPdaNewInstructionData {
    pub proof: ValidityProof,
    pub address_tree_info: PackedAddressTreeInfo,
    pub output_state_tree_index: u8,
    pub system_accounts_offset: u8,
}
