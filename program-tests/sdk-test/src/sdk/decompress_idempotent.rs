use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::DataHasher;
use light_sdk::{
    account::LightAccount,
    cpi::{CpiAccounts, CpiInputs},
    error::LightSdkError,
    instruction::{account_meta::CompressedAccountMeta, ValidityProof},
    LightDiscriminator,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, msg, program::invoke_signed, pubkey::Pubkey,
    rent::Rent, system_instruction, sysvar::Sysvar,
};

use crate::sdk::compress_pda::PdaTimingData;

pub const SLOTS_UNTIL_COMPRESSION: u64 = 100;

/// Helper function to decompress a compressed account into a PDA idempotently.
///
/// This function is idempotent, meaning it can be called multiple times with the same compressed account
/// and it will only decompress it once. If the PDA already exists and is initialized, it returns early.
///
/// # Arguments
/// * `pda_account` - The PDA account to decompress into
/// * `compressed_account_meta` - Optional metadata for the compressed account (None if PDA already exists)
/// * `compressed_account_data` - The data to write to the PDA
/// * `proof` - Optional validity proof (None if PDA already exists)
/// * `cpi_accounts` - Accounts needed for CPI
/// * `owner_program` - The program that will own the PDA
/// * `rent_payer` - The account to pay for PDA rent
/// * `system_program` - The system program
/// * `custom_seeds` - Custom seeds for PDA derivation (without the compressed address)
/// * `additional_seed` - Additional seed for PDA derivation
///
/// # Returns
/// * `Ok(())` if the compressed account was decompressed successfully or PDA already exists
/// * `Err(LightSdkError)` if there was an error
pub fn decompress_idempotent<'info, A>(
    pda_account: &AccountInfo<'info>,
    compressed_account_meta: Option<&CompressedAccountMeta>,
    compressed_account_data: &A,
    proof: ValidityProof,
    cpi_accounts: CpiAccounts<'_, 'info>,
    owner_program: &Pubkey,
    rent_payer: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    custom_seeds: &[&[u8]],
    additional_seed: &[u8; 32],
) -> Result<(), LightSdkError>
where
    A: DataHasher
        + LightDiscriminator
        + BorshSerialize
        + BorshDeserialize
        + Default
        + Clone
        + PdaTimingData,
{
    // Check if PDA is already initialized
    if pda_account.data_len() > 0 {
        msg!("PDA already initialized, skipping decompression");
        return Ok(());
    }

    // we zero out the compressed account.
    let mut compressed_account = LightAccount::<'_, A>::new_mut(
        owner_program,
        compressed_account_meta.ok_or(LightSdkError::ConstraintViolation)?,
        compressed_account_data.clone(), // TODO: try avoid clone
    )?;

    // Get compressed address
    let compressed_address = compressed_account
        .address()
        .ok_or(LightSdkError::ConstraintViolation)?;

    // Derive onchain PDA
    // CHECK: PDA is derived from compressed account address.
    let mut seeds: Vec<&[u8]> = custom_seeds.to_vec();
    seeds.push(&compressed_address);
    seeds.push(additional_seed);
    let (pda_pubkey, pda_bump) = Pubkey::find_program_address(&seeds, owner_program); // TODO: consider passing the bump.

    // Verify PDA matches
    if pda_pubkey != *pda_account.key {
        msg!("Invalid PDA pubkey");
        return Err(LightSdkError::ConstraintViolation);
    }

    // Get current slot
    let clock = Clock::get().map_err(|_| LightSdkError::Borsh)?;
    let current_slot = clock.slot;

    // Calculate space needed for PDA
    let space = std::mem::size_of::<A>() + 8; // +8 for discriminator

    // Get minimum rent
    let rent = Rent::get().map_err(|_| LightSdkError::Borsh)?;
    let minimum_balance = rent.minimum_balance(space);

    // Create PDA account
    let create_account_ix = system_instruction::create_account(
        rent_payer.key,
        pda_account.key,
        minimum_balance,
        space as u64,
        owner_program,
    );

    // Add bump to seeds for signing
    let bump_seed = [pda_bump];
    let mut signer_seeds = seeds.clone();
    signer_seeds.push(&bump_seed);
    let signer_seeds_refs: Vec<&[u8]> = signer_seeds.iter().map(|s| *s).collect();

    invoke_signed(
        &create_account_ix,
        &[
            rent_payer.clone(),
            pda_account.clone(),
            system_program.clone(),
        ],
        &[&signer_seeds_refs],
    )?;

    // Serialize the account data
    let mut data_bytes = vec![];
    compressed_account_data
        .serialize(&mut data_bytes)
        .map_err(|_| LightSdkError::Borsh)?;

    // Initialize PDA with decompressed data
    let mut decompressed_pda: A = compressed_account.account;
    decompressed_pda.set_last_written_slot(current_slot);

    // Write data to PDA
    decompressed_pda
        .serialize(&mut &mut pda_account.try_borrow_mut_data()?[8..])
        .map_err(|_| LightSdkError::Borsh)?;

    // Zero the compressed account with CPI
    compressed_account.account = A::default();
    let cpi_inputs = CpiInputs::new(proof, vec![compressed_account.to_account_info()?]);
    cpi_inputs.invoke_light_system_program(cpi_accounts)?;

    drop(pda_account.try_borrow_mut_data()?); // todo: check if this is needed.

    Ok(())
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct DecompressToPdaInstructionData {
    pub proof: ValidityProof,
    pub compressed_account: DecompressMyCompressedAccount,
    pub additional_seed: [u8; 32], // Additional seed for PDA derivation
    pub system_accounts_offset: u8,
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct DecompressMyCompressedAccount {
    pub meta: CompressedAccountMeta,
    pub data: [u8; 31],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decompress_to_pda::MyPdaAccount;
    use light_sdk::cpi::CpiAccountsConfig;

    /// Test instruction that demonstrates idempotent decompression
    /// This can be called multiple times with the same compressed account
    pub fn test_decompress_idempotent(
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> Result<(), LightSdkError> {
        msg!("Testing idempotent decompression");

        #[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
        struct TestInstructionData {
            pub proof: ValidityProof,
            pub compressed_account_meta: Option<CompressedAccountMeta>,
            pub data: [u8; 31],
            pub additional_seed: [u8; 32],
            pub system_accounts_offset: u8,
        }

        let mut instruction_data = instruction_data;
        let instruction_data = TestInstructionData::deserialize(&mut instruction_data)
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

        // Custom seeds
        let custom_seeds: Vec<&[u8]> = vec![b"decompressed_pda"];

        // Call decompress_idempotent - this should work whether PDA exists or not
        decompress_idempotent::<MyPdaAccount>(
            pda_account,
            instruction_data.compressed_account_meta.as_ref(),
            &account_data,
            instruction_data.proof,
            cpi_accounts,
            &crate::ID,
            rent_payer,
            system_program,
            &custom_seeds,
            &instruction_data.additional_seed,
        )?;

        msg!("Idempotent decompression completed successfully");
        Ok(())
    }
}
