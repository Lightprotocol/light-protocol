use borsh::{BorshDeserialize, BorshSerialize};
use light_macros::pubkey;
use light_sdk::{
    compressible::compress_pda_new,
    cpi::CpiAccounts,
    error::LightSdkError,
    instruction::{PackedAddressTreeInfo, ValidityProof},
};
use light_sdk_types::CpiAccountsConfig;
use solana_program::account_info::AccountInfo;
use solana_program::pubkey::Pubkey;

use crate::decompress_dynamic_pda::MyPdaAccount;

pub const ADDRESS_SPACE: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");
pub const RENT_RECIPIENT: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");

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
    // CHECK: hardcoded rent recipient.
    let rent_recipient = &accounts[2];
    if rent_recipient.key != &RENT_RECIPIENT {
        return Err(LightSdkError::ConstraintViolation);
    }

    // Cpi accounts
    let cpi_accounts_struct = CpiAccounts::new_with_config(
        fee_payer,
        &accounts[3..],
        CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER),
    );

    // the onchain PDA is the seed for the cPDA. this way devs don't have to
    // change their onchain PDA checks.
    let new_address_params = instruction_data
        .address_tree_info
        .into_new_address_params_packed(pda_account.key.to_bytes());

    compress_pda_new::<MyPdaAccount>(
        pda_account,
        instruction_data.compressed_address,
        new_address_params,
        instruction_data.output_state_tree_index,
        instruction_data.proof,
        cpi_accounts_struct,
        &crate::ID,
        rent_recipient,
        &ADDRESS_SPACE, // TODO: consider passing a slice of pubkeys, and extend to read_only_address_proofs.
    )?;

    Ok(())
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct CreateDynamicPdaInstructionData {
    pub proof: ValidityProof,
    pub compressed_address: [u8; 32],
    pub address_tree_info: PackedAddressTreeInfo,
    pub output_state_tree_index: u8,
}
