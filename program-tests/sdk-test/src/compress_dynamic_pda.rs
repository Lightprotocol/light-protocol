use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    cpi::CpiAccounts,
    error::LightSdkError,
    instruction::{account_meta::CompressedAccountMeta, ValidityProof},
};
use light_sdk_types::CpiAccountsConfig;
use solana_program::account_info::AccountInfo;

use crate::{
    create_dynamic_pda::RENT_RECIPIENT, decompress_dynamic_pda::MyPdaAccount,
    sdk::compress_pda::compress_pda,
};

/// Compresses a PDA back into a compressed account
/// Anyone can call this after the timeout period has elapsed
// TODO: add macro that create the full instruction. and takes: programid, account and seeds, rent_recipient (to hardcode). low code solution.
pub fn compress_dynamic_pda(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    let mut instruction_data = instruction_data;
    let instruction_data = CompressFromPdaInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;

    let pda_account = &accounts[1];

    // CHECK: hardcoded rent recipient.
    let rent_recipient = &accounts[2];
    if rent_recipient.key != &RENT_RECIPIENT {
        return Err(LightSdkError::ConstraintViolation);
    }

    // Cpi accounts
    let config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
    let cpi_accounts_struct = CpiAccounts::new_with_config(
        &accounts[0],
        &accounts[instruction_data.system_accounts_offset as usize..],
        config,
    );

    compress_pda::<MyPdaAccount>(
        pda_account,
        &instruction_data.compressed_account_meta,
        instruction_data.proof,
        cpi_accounts_struct,
        &crate::ID,
        rent_recipient,
    )?;

    // any other program logic here...

    Ok(())
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct CompressFromPdaInstructionData {
    pub proof: ValidityProof,
    pub compressed_account_meta: CompressedAccountMeta,
    pub system_accounts_offset: u8,
}
