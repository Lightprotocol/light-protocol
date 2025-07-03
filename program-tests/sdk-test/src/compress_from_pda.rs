use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    cpi::CpiAccounts,
    error::LightSdkError,
    instruction::{account_meta::CompressedAccountMeta, ValidityProof},
};
use light_sdk_types::CpiAccountsConfig;
use solana_program::account_info::AccountInfo;

use crate::{decompress_to_pda::DecompressedPdaAccount, sdk::compress_pda::compress_pda};

/// Compresses a PDA back into a compressed account
/// Anyone can call this after the timeout period has elapsed
/// pda check missing yet.
pub fn compress_from_pda(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    let mut instruction_data = instruction_data;
    let instruction_data = CompressFromPdaInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;

    // based on program...
    let custom_seeds: Vec<&[u8]> = vec![b"decompressed_pda"];

    let pda_account = &accounts[1];
    let rent_recipient = &accounts[2]; // can be hardcoded by caller program

    // Cpi accounts
    let config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
    let cpi_accounts_struct = CpiAccounts::new_with_config(
        &accounts[0],
        &accounts[instruction_data.system_accounts_offset as usize..],
        config,
    );

    compress_pda::<DecompressedPdaAccount>(
        pda_account,
        &instruction_data.compressed_account_meta,
        Some(instruction_data.proof),
        cpi_accounts_struct,
        &crate::ID,
        rent_recipient,
        &custom_seeds,
    )?;

    // any other program logic here...

    Ok(())
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct CompressFromPdaInstructionData {
    pub proof: ValidityProof,
    pub compressed_account_meta: CompressedAccountMeta,
    pub additional_seed: [u8; 32], // Must match the seed used in decompression
    pub system_accounts_offset: u8,
}
