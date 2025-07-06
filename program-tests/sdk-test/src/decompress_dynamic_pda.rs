use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    account::LightAccount,
    compressible::{decompress_idempotent, PdaTimingData, SLOTS_UNTIL_COMPRESSION},
    cpi::{CpiAccounts, CpiAccountsConfig},
    error::LightSdkError,
    instruction::{account_meta::CompressedAccountMeta, ValidityProof},
    LightDiscriminator, LightHasher,
};
use solana_program::account_info::AccountInfo;

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
    let rent_payer = &accounts[2];
    let system_program = &accounts[3];

    // Set up CPI accounts
    let mut config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
    config.sol_pool_pda = false;
    config.sol_compression_recipient = false;

    let cpi_accounts = CpiAccounts::new_with_config(
        fee_payer,
        &accounts[instruction_data.system_accounts_offset as usize..],
        config,
    );

    // Prepare account data
    let account_data = MyPdaAccount {
        last_written_slot: 0,
        slots_until_compression: SLOTS_UNTIL_COMPRESSION,
        data: instruction_data.data,
    };

    let compressed_account = LightAccount::<'_, MyPdaAccount>::new_mut(
        &crate::ID,
        &instruction_data.compressed_account_meta,
        account_data,
    )?;

    // Call decompress_idempotent - this should work whether PDA exists or not
    decompress_idempotent::<MyPdaAccount>(
        pda_account,
        compressed_account,
        instruction_data.proof,
        cpi_accounts,
        &crate::ID,
        rent_payer,
        system_program,
    )?;

    Ok(())
}

/// Example: Decompresses multiple compressed accounts into PDAs in a single transaction.
pub fn decompress_multiple_dynamic_pdas(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    use light_sdk::compressible::decompress_multiple_idempotent;

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

    // Get PDA accounts (after fixed accounts, before system accounts)
    let pda_accounts_start = 3;
    let pda_accounts_end = instruction_data.system_accounts_offset as usize;
    let pda_accounts = &accounts[pda_accounts_start..pda_accounts_end];

    // Set up CPI accounts
    let mut config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
    config.sol_pool_pda = false;
    config.sol_compression_recipient = false;

    let cpi_accounts = CpiAccounts::new_with_config(
        fee_payer,
        &accounts[instruction_data.system_accounts_offset as usize..],
        config,
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

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct DecompressToPdaInstructionData {
    pub proof: ValidityProof,
    pub compressed_account_meta: CompressedAccountMeta,
    pub data: [u8; 31],
    pub system_accounts_offset: u8,
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct MyCompressedAccount {
    pub meta: CompressedAccountMeta,
    pub data: MyPdaAccount,
}

#[derive(
    Clone, Debug, Default, LightHasher, LightDiscriminator, BorshDeserialize, BorshSerialize,
)]
pub struct MyPdaAccount {
    pub last_written_slot: u64,
    pub slots_until_compression: u64,
    pub data: [u8; 31],
}

// Implement the PdaTimingData trait
impl PdaTimingData for MyPdaAccount {
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
