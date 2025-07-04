use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    cpi::{CpiAccounts, CpiAccountsConfig},
    error::LightSdkError,
    instruction::{account_meta::CompressedAccountMeta, ValidityProof},
    LightDiscriminator, LightHasher,
};
use solana_program::account_info::AccountInfo;

use crate::sdk::decompress_idempotent::decompress_idempotent;

pub const SLOTS_UNTIL_COMPRESSION: u64 = 100;

/// Decompresses a compressed account into a PDA idempotently.
pub fn decompress_to_pda(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    let mut instruction_data = instruction_data;
    let instruction_data = DecompressToPdaInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;

    // Get accounts
    let fee_payer = &accounts[0];
    let pda_account = &accounts[1];
    let rent_payer = &accounts[2]; // Account that pays for PDA rent
    let system_program = &accounts[3];

    // Cpi accounts
    let cpi_accounts = CpiAccounts::new_with_config(
        fee_payer,
        &accounts[instruction_data.system_accounts_offset as usize..],
        CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER),
    );

    // Custom seeds for PDA derivation
    let custom_seeds: Vec<&[u8]> = vec![b"decompressed_pda"];

    // Call the SDK function to decompress idempotently
    // this inits pda_account if not already initialized
    decompress_idempotent::<MyPdaAccount>(
        pda_account,
        Some(&instruction_data.compressed_account.meta),
        &instruction_data.compressed_account.data,
        instruction_data.proof,
        cpi_accounts,
        &crate::ID,
        rent_payer,
        system_program,
        &custom_seeds,
        &instruction_data.additional_seed,
    )?;

    // do something with pda_account...

    Ok(())
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct DecompressToPdaInstructionData {
    pub proof: ValidityProof,
    pub compressed_account: MyCompressedAccount,
    pub additional_seed: [u8; 32], // ... some seed
    pub system_accounts_offset: u8,
}

// just a wrapper
#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct MyCompressedAccount {
    pub meta: CompressedAccountMeta,
    pub data: MyPdaAccount,
}

/// Account structure for the PDA
#[derive(
    Clone, Debug, LightHasher, LightDiscriminator, Default, BorshDeserialize, BorshSerialize,
)]
pub struct MyPdaAccount {
    /// Slot when this account was last written
    pub last_written_slot: u64,
    /// Number of slots after last_written_slot until this account can be compressed again
    pub slots_until_compression: u64,
    /// The actual account data
    pub data: [u8; 31],
}

// We require this trait to be implemented for the custom PDA account.
impl crate::sdk::compress_pda::PdaTimingData for MyPdaAccount {
    fn last_touched_slot(&self) -> u64 {
        self.last_written_slot
    }

    fn slots_buffer(&self) -> u64 {
        self.slots_until_compression
    }

    fn set_last_written_slot(&mut self, slot: u64) {
        self.last_written_slot = slot;
    }
}
