use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    account::LightAccount,
    compressible::{
        prepare_accounts_for_decompress_idempotent, CompressibleConfig, CompressionInfo,
        HasCompressionInfo,
    },
    cpi::{CpiAccounts, CpiInputs},
    error::LightSdkError,
    instruction::{account_meta::CompressedAccountMeta, ValidityProof},
    LightDiscriminator, LightHasher,
};
use solana_program::account_info::AccountInfo;
use solana_program::msg;

#[derive(Clone, Debug, BorshDeserialize, BorshSerialize)]
pub struct CompressedAccountData<T> {
    pub meta: CompressedAccountMeta,
    /// Program-specific account variant enum
    pub data: T,
}
/// Example: Decompresses multiple compressed accounts into PDAs in a single transaction.
pub fn decompress_multiple_dynamic_pdas(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    #[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
    pub struct DecompressMultipleInstructionData {
        pub proof: ValidityProof,
        // pub compressed_accounts: Vec<MyCompressedAccount>,
        pub compressed_accounts: Vec<CompressedAccountData<MyPdaAccount>>,
        pub bumps: Vec<u8>,
        pub system_accounts_offset: u8,
    }

    let mut instruction_data = instruction_data;
    let instruction_data = DecompressMultipleInstructionData::deserialize(&mut instruction_data)
        .map_err(|e| {
            solana_program::msg!(
                "Failed to deserialize DecompressMultipleInstructionData: {:?}",
                e
            );
            LightSdkError::Borsh
        })?;

    msg!("decompress_multiple_dynamic_pdas accounts: {:?}", accounts);

    // Account structure from CompressibleInstruction:
    // [0] fee_payer (signer)
    // [1] rent_payer (signer)
    // [2] system_program
    // [3..3+system_accounts_offset] PDA accounts
    // [3+system_accounts_offset..] Light Protocol system accounts

    let fee_payer = &accounts[0];
    let rent_payer = &accounts[1];
    let config_account = &accounts[2];
    let config = CompressibleConfig::load_checked(config_account, &crate::ID)?;

    // PDA accounts start at index 3 and go for system_accounts_offset accounts
    let pda_accounts_start = 3;
    let pda_accounts_end = pda_accounts_start + instruction_data.system_accounts_offset as usize;
    msg!("pda_accounts_start: {:?}", pda_accounts_start);
    msg!("pda_accounts_end: {:?}", pda_accounts_end);
    let pda_accounts = &accounts[pda_accounts_start..pda_accounts_end];
    msg!("pda_accounts: {:?}", pda_accounts);

    // Light Protocol system accounts start after PDA accounts
    let system_accounts_start = pda_accounts_end;
    let cpi_accounts = CpiAccounts::new(
        fee_payer,
        &accounts[system_accounts_start..],
        crate::LIGHT_CPI_SIGNER,
    );

    // Validate we have matching number of PDAs, compressed accounts, and bumps
    if pda_accounts.len() != instruction_data.compressed_accounts.len()
        || pda_accounts.len() != instruction_data.bumps.len()
    {
        return Err(LightSdkError::ConstraintViolation);
    }

    // First pass: validate PDAs and collect data
    let mut compressed_accounts = Vec::new();
    let mut pda_account_refs = Vec::new();
    let stored_bumps = instruction_data.bumps.clone(); // Store bumps to avoid borrowing issues

    for (i, compressed_account_data) in instruction_data.compressed_accounts.iter().enumerate() {
        let compressed_account = LightAccount::<'_, MyPdaAccount>::new_mut(
            &crate::ID,
            &compressed_account_data.meta,
            compressed_account_data.data.clone(),
        )?;

        let bump = stored_bumps[i];

        // Derive PDA for verification using the provided bump
        let seeds: Vec<&[u8]> = vec![b"dynamic_pda"];
        let (derived_pda, expected_bump) =
            solana_program::pubkey::Pubkey::find_program_address(&seeds, &crate::ID);

        // Verify the PDA matches
        if derived_pda != *pda_accounts[i].key {
            msg!(
                "derived_pda: {:?} does not match passed pda: {:?}",
                derived_pda,
                pda_accounts[i].key
            );
            return Err(LightSdkError::ConstraintViolation);
        }

        // Verify the provided bump matches the expected bump
        if bump != expected_bump {
            msg!(
                "provided bump: {:?}, expected bump: {:?}",
                bump,
                expected_bump
            );
            return Err(LightSdkError::ConstraintViolation);
        }

        compressed_accounts.push(compressed_account);
        pda_account_refs.push(&pda_accounts[i]);
    }

    // Second pass: build signer seeds with stable references
    let mut all_signer_seeds = Vec::new();
    for i in 0..stored_bumps.len() {
        let signer_seeds = vec![b"dynamic_pda" as &[u8], &stored_bumps[i..i + 1] as &[u8]];
        all_signer_seeds.push(signer_seeds);
    }

    // Convert to the format needed by the SDK
    let signer_seeds_refs: Vec<&[&[u8]]> = all_signer_seeds
        .iter()
        .map(|seeds| seeds.as_slice())
        .collect();

    // For sdk-test, we'll use a hardcoded address space that matches the test setup
    // This should match the address space used in tests
    let address_space = config.address_space[0];

    // Use prepare_accounts_for_decompress_idempotent directly and handle CPI manually
    let compressed_infos = prepare_accounts_for_decompress_idempotent::<MyPdaAccount>(
        &pda_account_refs,
        compressed_accounts,
        &signer_seeds_refs,
        &cpi_accounts,
        &crate::ID,
        rent_payer,
        address_space,
    )?;

    if !compressed_infos.is_empty() {
        let cpi_inputs = CpiInputs::new(instruction_data.proof, compressed_infos);
        cpi_inputs.invoke_light_system_program(cpi_accounts)?;
    }

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
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    pub data: [u8; 31],
}

impl anchor_lang::Discriminator for MyPdaAccount {
    const DISCRIMINATOR: &'static [u8] = &[1; 8];
}

// Implement the HasCompressionInfo trait
impl HasCompressionInfo for MyPdaAccount {
    fn compression_info(&self) -> &CompressionInfo {
        self.compression_info
            .as_ref()
            .expect("CompressionInfo must be Some on-chain")
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        self.compression_info
            .as_mut()
            .expect("CompressionInfo must be Some on-chain")
    }

    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo> {
        &mut self.compression_info
    }

    fn set_compression_info_none(&mut self) {
        self.compression_info = None;
    }
}
