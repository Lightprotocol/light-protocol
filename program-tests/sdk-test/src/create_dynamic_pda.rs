use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::instruction_data::data::ReadOnlyAddress;
use light_sdk::{
    compressible::{compress_pda_new_with_data, CompressibleConfig, CompressionInfo},
    cpi::CpiAccounts,
    error::LightSdkError,
    instruction::{PackedAddressTreeInfo, ValidityProof},
};
use solana_program::account_info::AccountInfo;

use crate::decompress_dynamic_pda::MyPdaAccount;

/// INITS a PDA and compresses it into a new compressed account.
pub fn create_dynamic_pda(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    let mut instruction_data = instruction_data;
    let instruction_data = CreateDynamicPdaInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;

    let fee_payer = &accounts[0];
    // UNCHECKED: ...caller program checks this.
    let pda_account = &accounts[1];
    let rent_recipient = &accounts[2];
    let config_account = &accounts[3];

    // Load config
    let config = CompressibleConfig::load_checked(config_account, &crate::ID)?;

    // CHECK: rent recipient from config
    if rent_recipient.key != &config.rent_recipient {
        return Err(LightSdkError::ConstraintViolation);
    }

    // Cpi accounts
    let cpi_accounts_struct = CpiAccounts::new(fee_payer, &accounts[4..], crate::LIGHT_CPI_SIGNER);

    // the onchain PDA is the seed for the cPDA. this way devs don't have to
    // change their onchain PDA checks.
    let new_address_params = instruction_data
        .address_tree_info
        .into_new_address_params_packed(pda_account.key.to_bytes());

    // We do not have to serialize into the PDA account, it's closed at the end
    // of this invocation.
    let mut pda_account_data = MyPdaAccount::try_from_slice(&pda_account.data.borrow())
        .map_err(|_| LightSdkError::Borsh)?;

    // Initialize compression info with current slot and decompressed state
    pda_account_data.compression_info = CompressionInfo::new()?;

    // Use the efficient native variant that accepts pre-deserialized data
    compress_pda_new_with_data::<MyPdaAccount>(
        pda_account,
        &mut pda_account_data,
        instruction_data.compressed_address,
        new_address_params,
        instruction_data.output_state_tree_index,
        instruction_data.proof,
        cpi_accounts_struct,
        &crate::ID,
        rent_recipient,
        &config.address_space,
        instruction_data.read_only_addresses,
    )?;

    Ok(())
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct CreateDynamicPdaInstructionData {
    pub proof: ValidityProof,
    pub compressed_address: [u8; 32],
    pub address_tree_info: PackedAddressTreeInfo,
    /// Optional read-only addresses for exclusion proofs (same address, different trees)
    pub read_only_addresses: Option<Vec<ReadOnlyAddress>>,
    pub output_state_tree_index: u8,
}
