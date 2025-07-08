use crate::{
    account::LightAccount,
    cpi::{CpiAccounts, CpiInputs},
    error::LightSdkError,
    instruction::{account_meta::CompressedAccountMeta, ValidityProof},
    LightDiscriminator,
};
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize as BorshDeserialize, AnchorSerialize as BorshSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::DataHasher;
use solana_account_info::AccountInfo;
use solana_clock::Clock;
use solana_cpi::invoke_signed;
use solana_msg::msg;
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use solana_system_interface::instruction as system_instruction;
use solana_sysvar::Sysvar;

use crate::compressible::compress_pda::PdaTimingData;

pub const SLOTS_UNTIL_COMPRESSION: u64 = 100;

/// Helper function to decompress a compressed account into a PDA idempotently.
///
/// This function is idempotent, meaning it can be called multiple times with the same compressed account
/// and it will only decompress it once. If the PDA already exists and is initialized, it returns early.
///
/// # Arguments
/// * `pda_account` - The PDA account to decompress into
/// * `compressed_account` - The compressed account to decompress
/// * `proof` - Validity proof
/// * `cpi_accounts` - Accounts needed for CPI
/// * `owner_program` - The program that will own the PDA
/// * `rent_payer` - The account to pay for PDA rent
/// * `system_program` - The system program
///
/// # Returns
/// * `Ok(())` if the compressed account was decompressed successfully or PDA already exists
/// * `Err(LightSdkError)` if there was an error
pub fn decompress_idempotent<'info, A>(
    pda_account: &AccountInfo<'info>,
    compressed_account: LightAccount<'_, A>,
    proof: ValidityProof,
    cpi_accounts: CpiAccounts<'_, 'info>,
    owner_program: &Pubkey,
    rent_payer: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
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
    decompress_multiple_idempotent(
        &[pda_account],
        vec![compressed_account],
        proof,
        cpi_accounts,
        owner_program,
        rent_payer,
        system_program,
    )
}

/// Helper function to decompress multiple compressed accounts into PDAs idempotently.
///
/// This function is idempotent, meaning it can be called multiple times with the same compressed accounts
/// and it will only decompress them once. If a PDA already exists and is initialized, it skips that account.
///
/// # Arguments
/// * `pda_accounts` - The PDA accounts to decompress into
/// * `compressed_accounts` - The compressed accounts to decompress
/// * `proof` - Single validity proof for all accounts
/// * `cpi_accounts` - Accounts needed for CPI
/// * `owner_program` - The program that will own the PDAs
/// * `rent_payer` - The account to pay for PDA rent
/// * `system_program` - The system program
///
/// # Returns
/// * `Ok(())` if all compressed accounts were decompressed successfully or PDAs already exist
/// * `Err(LightSdkError)` if there was an error
pub fn decompress_multiple_idempotent<'info, A>(
    pda_accounts: &[&AccountInfo<'info>],
    compressed_accounts: Vec<LightAccount<'_, A>>,
    proof: ValidityProof,
    cpi_accounts: CpiAccounts<'_, 'info>,
    owner_program: &Pubkey,
    rent_payer: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
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
    // Get current slot and rent once for all accounts
    let clock = Clock::get().map_err(|_| LightSdkError::Borsh)?;
    let current_slot = clock.slot;
    let rent = Rent::get().map_err(|_| LightSdkError::Borsh)?;

    // Calculate space needed for PDA (same for all accounts of type A)
    let space = std::mem::size_of::<A>() + 8; // +8 for discriminator
    let rent_minimum_balance = rent.minimum_balance(space);

    // Collect compressed accounts for CPI
    let mut compressed_accounts_for_cpi = Vec::new();

    for (pda_account, mut compressed_account) in
        pda_accounts.iter().zip(compressed_accounts.into_iter())
    {
        // Check if PDA is already initialized
        if pda_account.data_len() > 0 {
            msg!(
                "PDA {} already initialized, skipping decompression",
                pda_account.key
            );
            continue;
        }

        // Get the compressed account address
        let compressed_address = compressed_account
            .address()
            .ok_or(LightSdkError::ConstraintViolation)?;

        // Derive onchain PDA using the compressed address as seed
        let seeds: Vec<&[u8]> = vec![&compressed_address];

        let (pda_pubkey, pda_bump) = Pubkey::find_program_address(&seeds, owner_program);

        // Verify PDA matches
        if pda_pubkey != *pda_account.key {
            msg!("Invalid PDA pubkey for account {}", pda_account.key);
            return Err(LightSdkError::ConstraintViolation);
        }

        // Create PDA account
        let create_account_ix = system_instruction::create_account(
            rent_payer.key,
            pda_account.key,
            rent_minimum_balance,
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
                (*pda_account).clone(),
                system_program.clone(),
            ],
            &[&signer_seeds_refs],
        )?;

        // Initialize PDA with decompressed data and update slot
        let mut decompressed_pda = compressed_account.account.clone();
        decompressed_pda.set_last_written_slot(current_slot);

        // Write discriminator
        let discriminator = A::LIGHT_DISCRIMINATOR;
        pda_account.try_borrow_mut_data()?[..8].copy_from_slice(&discriminator);

        // Write data to PDA
        decompressed_pda
            .serialize(&mut &mut pda_account.try_borrow_mut_data()?[8..])
            .map_err(|_| LightSdkError::Borsh)?;

        // Zero the compressed account
        compressed_account.account = A::default();

        // Add to CPI batch
        compressed_accounts_for_cpi.push(compressed_account.to_account_info()?);
    }

    // Make single CPI call with all compressed accounts
    if !compressed_accounts_for_cpi.is_empty() {
        let cpi_inputs = CpiInputs::new(proof, compressed_accounts_for_cpi);
        cpi_inputs.invoke_light_system_program(cpi_accounts)?;
    }

    Ok(())
}
