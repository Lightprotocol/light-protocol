use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    account::LightAccount,
    cpi::{CpiAccounts, CpiAccountsConfig},
    error::LightSdkError,
    instruction::{account_meta::CompressedAccountMeta, ValidityProof},
    LightDiscriminator, LightHasher,
};
use solana_program::account_info::AccountInfo;

use crate::sdk::decompress_idempotent::decompress_idempotent;

pub const SLOTS_UNTIL_COMPRESSION: u64 = 10_000;

/// Decompresses a compressed account into a PDA idempotently.
pub fn decompress_dynamic_pda(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    let mut instruction_data = instruction_data;
    let instruction_data = DecompressToPdaInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;

    // Get accounts
    let fee_payer = &accounts[0];
    let pda_account = &accounts[1];
    let rent_payer = &accounts[2]; // Anyone can pay.
    let system_program = &accounts[3];

    // Cpi accounts
    let cpi_accounts = CpiAccounts::new_with_config(
        fee_payer,
        &accounts[instruction_data.system_accounts_offset as usize..],
        CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER),
    );
    // we zero out the compressed account.
    let compressed_account = LightAccount::<'_, MyPdaAccount>::new_mut(
        &crate::ID,
        &instruction_data.compressed_account.meta,
        instruction_data.compressed_account.data,
    )?;

    // Call the SDK function to decompress idempotently
    // this inits pda_account if not already initialized
    decompress_idempotent::<MyPdaAccount>(
        pda_account,
        compressed_account,
        instruction_data.proof,
        cpi_accounts,
        &crate::ID,
        rent_payer,
        system_program,
    )?;

    // do something with pda_account...

    Ok(())
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct DecompressToPdaInstructionData {
    pub proof: ValidityProof,
    pub compressed_account: MyCompressedAccount,
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
    /// Number of slots after last_written_slot until this account can be
    /// compressed again
    pub slots_until_compression: u64,
    /// The actual account data
    pub data: [u8; 31],
}

// We require this trait to be implemented for the custom PDA account.
impl crate::sdk::compress_pda::PdaTimingData for MyPdaAccount {
    fn last_written_slot(&self) -> u64 {
        self.last_written_slot
    }

    fn slots_until_compression(&self) -> u64 {
        self.slots_until_compression
    }

    fn set_last_written_slot(&mut self, slot: u64) {
        self.last_written_slot = slot;
    }
}

// TODO: do this properly.
pub fn decompress_multiple_dynamic_pdas(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    use crate::sdk::decompress_idempotent::decompress_multiple_idempotent;

    #[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
    pub struct DecompressMultipleInstructionData {
        pub proof: ValidityProof,
        pub compressed_accounts: Vec<MyCompressedAccount>,
        pub system_accounts_offset: u8,
    }

    let mut instruction_data = instruction_data;
    let instruction_data = DecompressMultipleInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;

    // Get fixed accounts
    let fee_payer = &accounts[0];
    let rent_payer = &accounts[1];
    let system_program = &accounts[2];

    // Calculate where PDA accounts start
    let pda_accounts_start = 3;
    let num_accounts = instruction_data.compressed_accounts.len();

    // Get PDA accounts
    let pda_accounts = &accounts[pda_accounts_start..pda_accounts_start + num_accounts];

    // Cpi accounts
    // TODO: currently all cPDAs would have to have the same CPI_ACCOUNTS in the same order.
    // - must support flexible CPI_ACCOUNTS eg for token accounts
    // - must support flexible trees.
    let cpi_accounts = CpiAccounts::new_with_config(
        fee_payer,
        &accounts[instruction_data.system_accounts_offset as usize..],
        CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER),
    );

    // Build inputs for batch decompression
    let mut compressed_accounts = Vec::new();
    let mut pda_account_refs = Vec::new();

    for (i, compressed_account_data) in instruction_data.compressed_accounts.into_iter().enumerate()
    {
        let compressed_account = LightAccount::<'_, MyPdaAccount>::new_mut(
            &crate::ID,
            &compressed_account_data.meta,
            compressed_account_data.data,
        )?;

        compressed_accounts.push(compressed_account);
        pda_account_refs.push(&pda_accounts[i]);
    }

    // Decompress all accounts in one CPI call
    decompress_multiple_idempotent::<MyPdaAccount>(
        &pda_account_refs,
        compressed_accounts,
        instruction_data.proof,
        cpi_accounts,
        &crate::ID,
        rent_payer,
        system_program,
    )?;

    Ok(())
}
