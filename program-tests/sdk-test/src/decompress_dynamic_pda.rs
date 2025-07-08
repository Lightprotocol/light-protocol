use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    account::LightAccount,
    compressible::{decompress_idempotent, CompressionTiming},
    cpi::{CpiAccounts, CpiAccountsConfig},
    error::LightSdkError,
    instruction::{account_meta::CompressedAccountMeta, ValidityProof},
    LightDiscriminator, LightHasher,
};
use solana_program::account_info::AccountInfo;

pub const COMPRESSION_DELAY: u64 = 100;

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
    let config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
    let cpi_accounts = CpiAccounts::new_with_config(
        fee_payer,
        &accounts[instruction_data.system_accounts_offset as usize..],
        config,
    );

    let compressed_account = LightAccount::<'_, MyPdaAccount>::new_mut(
        &crate::ID,
        &instruction_data.compressed_account.meta,
        instruction_data.compressed_account.data,
    )?;

    // Extract the data field for use in seeds
    let account_data = compressed_account.data;

    // Derive the PDA seeds and bump
    // In a real implementation, you would pass these as part of the instruction data
    // For this example, we'll use the account data as part of the seed
    let seeds: &[&[u8]] = &[b"test_pda", &account_data];
    let (derived_pda, bump) =
        solana_program::pubkey::Pubkey::find_program_address(seeds, &crate::ID);

    // Verify the PDA matches
    if derived_pda != *pda_account.key {
        return Err(LightSdkError::ConstraintViolation);
    }

    // Call decompress_idempotent with seeds - this should work whether PDA exists or not
    decompress_idempotent::<MyPdaAccount>(
        pda_account,
        compressed_account,
        seeds,
        bump,
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
    let mut all_seeds = Vec::new();
    let mut bumps = Vec::new();

    for (i, compressed_account_data) in instruction_data.compressed_accounts.into_iter().enumerate()
    {
        let compressed_account = LightAccount::<'_, MyPdaAccount>::new_mut(
            &crate::ID,
            &compressed_account_data.meta,
            compressed_account_data.data.clone(),
        )?;

        // Store seeds in a vector to ensure they live long enough
        all_seeds.push(vec![
            b"test_pda".to_vec(),
            compressed_account_data.data.data.to_vec(),
        ]);

        // Create references to the seeds
        let seeds: Vec<&[u8]> = all_seeds
            .last()
            .unwrap()
            .iter()
            .map(|s| s.as_slice())
            .collect();
        let (derived_pda, bump) =
            solana_program::pubkey::Pubkey::find_program_address(&seeds, &crate::ID);

        // Verify the PDA matches
        if derived_pda != *pda_accounts[i].key {
            return Err(LightSdkError::ConstraintViolation);
        }

        compressed_accounts.push(compressed_account);
        pda_account_refs.push(&pda_accounts[i]);
        bumps.push(bump);
    }

    // Create seeds references for the function call
    let seeds_refs: Vec<Vec<&[u8]>> = all_seeds
        .iter()
        .map(|seeds| seeds.iter().map(|s| s.as_slice()).collect())
        .collect();
    let seeds_list: Vec<&[&[u8]]> = seeds_refs.iter().map(|seeds| seeds.as_slice()).collect();

    // Decompress all accounts in one CPI call
    decompress_multiple_idempotent::<MyPdaAccount>(
        &pda_account_refs,
        compressed_accounts,
        &seeds_list,
        &bumps,
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
    pub compressed_account: MyCompressedAccount,
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
    pub compression_delay: u64,
    pub data: [u8; 31],
}

// Implement the CompressionTiming trait
impl CompressionTiming for MyPdaAccount {
    fn last_written_slot(&self) -> u64 {
        self.last_written_slot
    }

    fn compression_delay(&self) -> u64 {
        self.compression_delay
    }

    fn set_last_written_slot(&mut self, slot: u64) {
        self.last_written_slot = slot;
    }
}
