use crate::{
    account::LightAccount,
    cpi::{CpiAccounts, CpiInputs},
    error::LightSdkError,
    instruction::ValidityProof,
    LightDiscriminator,
};
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize as BorshDeserialize, AnchorSerialize as BorshSerialize};
use arrayvec::ArrayVec;
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::address::derive_address;
use light_hasher::DataHasher;
use solana_account_info::AccountInfo;
use solana_clock::Clock;
use solana_cpi::invoke_signed;
use solana_msg::msg;
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use solana_system_interface::instruction as system_instruction;
use solana_sysvar::Sysvar;

use crate::compressible::compress_pda::CompressionTiming;

pub const COMPRESSION_DELAY: u64 = 100;

/// Helper function to decompress a compressed account into a PDA idempotently with seeds.
///
/// This function is idempotent, meaning it can be called multiple times with the same compressed account
/// and it will only decompress it once. If the PDA already exists and is initialized, it returns early.
///
/// # Arguments
/// * `pda_account` - The PDA account to decompress into
/// * `compressed_account` - The compressed account to decompress
/// * `seeds` - The seeds used to derive the PDA
/// * `bump` - The bump seed for the PDA
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
    seeds: &[&[u8]],
    bump: u8,
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
        + CompressionTiming,
{
    decompress_multiple_idempotent(
        &[pda_account],
        vec![compressed_account],
        &[seeds],
        &[bump],
        proof,
        cpi_accounts,
        owner_program,
        rent_payer,
        system_program,
    )
}

/// Helper function to decompress multiple compressed accounts into PDAs idempotently with seeds.
///
/// This function is idempotent, meaning it can be called multiple times with the same compressed accounts
/// and it will only decompress them once. If a PDA already exists and is initialized, it skips that account.
///
/// # Arguments
/// * `pda_accounts` - The PDA accounts to decompress into
/// * `compressed_accounts` - The compressed accounts to decompress
/// * `seeds_list` - List of seeds for each PDA (one per account)
/// * `bumps` - List of bump seeds for each PDA (one per account)
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
    seeds_list: &[&[&[u8]]],
    bumps: &[u8],
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
        + CompressionTiming,
{
    // Validate input lengths
    if pda_accounts.len() != compressed_accounts.len()
        || pda_accounts.len() != seeds_list.len()
        || pda_accounts.len() != bumps.len()
    {
        return Err(LightSdkError::ConstraintViolation);
    }

    // Get current slot and rent once for all accounts
    let clock = Clock::get().map_err(|_| LightSdkError::Borsh)?;
    let current_slot = clock.slot;
    let rent = Rent::get().map_err(|_| LightSdkError::Borsh)?;

    let mut compressed_accounts_for_cpi = Vec::new();

    for (((pda_account, mut compressed_account), seeds), &bump) in pda_accounts
        .iter()
        .zip(compressed_accounts.into_iter())
        .zip(seeds_list.iter())
        .zip(bumps.iter())
    {
        // TODO: consider a COMPRESSED_DISCIMINATOR.
        // compress -> set compressed
        // decompress -> if INITED but compressed, decompress anyway.
        // Check if PDA is already initialized
        if pda_account.data_len() > 0 {
            msg!(
                "PDA {} already initialized, skipping decompression",
                pda_account.key
            );
            continue;
        }

        // Get the compressed account address
        let c_pda = compressed_account
            .address()
            .ok_or(LightSdkError::ConstraintViolation)?;

        let derived_c_pda = derive_address(
            &pda_account.key.to_bytes(),
            &compressed_account
                .get_tree_pubkey(&cpi_accounts)?
                .to_bytes(),
            &owner_program.to_bytes(),
        );
        // CHECK:
        // pda and c_pda are related
        if c_pda != derived_c_pda {
            msg!(
                "PDA and cPDA mismatch: {} {:?}",
                pda_account.key,
                derived_c_pda
            );
            return Err(LightSdkError::ConstraintViolation);
        }

        let space = compressed_account.size()?;
        let rent_minimum_balance = rent.minimum_balance(space);

        // Create PDA account
        let create_account_ix = system_instruction::create_account(
            rent_payer.key,
            pda_account.key,
            rent_minimum_balance,
            space as u64,
            owner_program,
        );

        // Add bump to seeds for signing
        let bump_seed = [bump];

        // Use ArrayVec to avoid heap allocation - Solana supports max 16 seeds
        let mut signer_seeds = ArrayVec::<&[u8], 16>::new();
        for seed in seeds.iter() {
            signer_seeds.push(*seed);
        }
        signer_seeds.push(&bump_seed);

        invoke_signed(
            &create_account_ix,
            &[
                rent_payer.clone(),
                (*pda_account).clone(),
                system_program.clone(),
            ],
            &[&signer_seeds],
        )?;

        // Initialize PDA with decompressed data and update slot
        let mut decompressed_pda = compressed_account.account.clone();
        decompressed_pda.set_last_written_slot(current_slot);

        // Write discriminator
        // TODO: we don't mind the onchain account being different?
        // TODO: consider passing onchain account discriminator? (can be auto-derived)
        let discriminator = A::LIGHT_DISCRIMINATOR;
        pda_account.try_borrow_mut_data()?[..8].copy_from_slice(&discriminator);

        // Write data to PDA
        decompressed_pda
            .serialize(&mut &mut pda_account.try_borrow_mut_data()?[8..])
            .map_err(|_| LightSdkError::Borsh)?;

        // Zero the compressed account data
        compressed_account.remove_data();

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
