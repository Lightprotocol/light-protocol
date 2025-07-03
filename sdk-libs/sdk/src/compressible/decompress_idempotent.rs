use crate::{
    account::LightAccount,
    compressible::compression_info::HasCompressionInfo,
    cpi::{CpiAccounts, CpiInputs},
    error::LightSdkError,
    instruction::ValidityProof,
    LightDiscriminator,
};

#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize as BorshDeserialize, AnchorSerialize as BorshSerialize};
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

#[cfg(feature = "anchor")]
use anchor_lang::Discriminator as AnchorDiscriminatorShim;

#[cfg(not(feature = "anchor"))]
trait AnchorDiscriminatorShim {}
#[cfg(not(feature = "anchor"))]
impl<T> AnchorDiscriminatorShim for T {}

#[cfg(feature = "anchor")]
use anchor_lang::{prelude::Account, AccountDeserialize, AccountSerialize, ToAccountInfo};

pub const COMPRESSION_DELAY: u64 = 100;

/// Helper function to decompress a compressed account into a PDA idempotently with seeds.
///
/// This function is idempotent, meaning it can be called multiple times with the same compressed account
/// and it will only decompress it once. If the PDA already exists and is initialized, it returns early.
///
/// # Arguments
/// * `pda_account` - The PDA account to decompress into
/// * `compressed_account` - The compressed account to decompress
/// * `signer_seeds` - The signer seeds including bump (standard Solana format)
/// * `proof` - Validity proof
/// * `cpi_accounts` - Accounts needed for CPI
/// * `owner_program` - The program that will own the PDA
/// * `rent_payer` - The account to pay for PDA rent
/// * `address_space` - The address space for the compressed account
///
/// # Returns
/// * `Ok(())` if the compressed account was decompressed successfully or PDA already exists
/// * `Err(LightSdkError)` if there was an error
pub fn decompress_idempotent<'info, T>(
    pda_account: &AccountInfo<'info>,
    compressed_account: LightAccount<'_, T>,
    signer_seeds: &[&[u8]],
    proof: ValidityProof,
    cpi_accounts: CpiAccounts<'_, 'info>,
    owner_program: &Pubkey,
    rent_payer: &AccountInfo<'info>,
    address_space: Pubkey,
) -> Result<(), LightSdkError>
where
    T: DataHasher
        + LightDiscriminator
        + BorshSerialize
        + BorshDeserialize
        + Default
        + Clone
        + HasCompressionInfo
        + AnchorDiscriminatorShim,
{
    decompress_multiple_idempotent(
        &[pda_account],
        vec![compressed_account],
        &[signer_seeds],
        proof,
        cpi_accounts,
        owner_program,
        rent_payer,
        address_space,
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
/// * `signer_seeds` - Signer seeds for each PDA including bump (standard Solana format)
/// * `proof` - Single validity proof for all accounts
/// * `cpi_accounts` - Accounts needed for CPI
/// * `owner_program` - The program that will own the PDAs
/// * `rent_payer` - The account to pay for PDA rent
/// * `address_space` - The address space for the compressed accounts
///
/// # Returns
/// * `Ok(())` if all compressed accounts were decompressed successfully or PDAs already exist
/// * `Err(LightSdkError)` if there was an error
pub fn decompress_multiple_idempotent<'info, T>(
    pda_accounts: &[&AccountInfo<'info>],
    compressed_accounts: Vec<LightAccount<'_, T>>,
    signer_seeds: &[&[&[u8]]],
    proof: ValidityProof,
    cpi_accounts: CpiAccounts<'_, 'info>,
    owner_program: &Pubkey,
    rent_payer: &AccountInfo<'info>,
    address_space: Pubkey,
) -> Result<(), LightSdkError>
where
    T: DataHasher
        + LightDiscriminator
        + BorshSerialize
        + BorshDeserialize
        + Default
        + Clone
        + HasCompressionInfo
        + AnchorDiscriminatorShim,
{
    // Validate input lengths
    if pda_accounts.len() != compressed_accounts.len() || pda_accounts.len() != signer_seeds.len() {
        return Err(LightSdkError::ConstraintViolation);
    }

    // Get current slot and rent once for all accounts
    let clock = Clock::get().map_err(|_| LightSdkError::Borsh)?;
    let current_slot = clock.slot;
    let rent = Rent::get().map_err(|_| LightSdkError::Borsh)?;

    let mut compressed_accounts_for_cpi = Vec::new();

    for ((pda_account, mut compressed_account), seeds) in pda_accounts
        .iter()
        .zip(compressed_accounts.into_iter())
        .zip(signer_seeds.iter())
    {
        // Check if PDA is already initialized
        if !pda_account.data_is_empty() {
            msg!(
                "PDA DATA {} already initialized, skipping decompression",
                pda_account.key
            );
            continue;
        }
        // TODO: remove
        // if **pda_account.lamports.borrow() > 0 {
        //     msg!(
        //         "PDA {} already initialized, skipping decompression",
        //         pda_account.key
        //     );
        //     continue;
        // }

        // Get the compressed account address
        let c_pda = compressed_account
            .address()
            .ok_or(LightSdkError::ConstraintViolation)?;

        let derived_c_pda = derive_address(
            &pda_account.key.to_bytes(),
            &address_space.to_bytes(),
            &owner_program.to_bytes(),
        );

        // CHECK:
        // pda and c_pda are related
        if c_pda != derived_c_pda {
            msg!(
                "cPDA {:?} does not match derived cPDA {:?} for PDA {:?} with address space {:?}",
                c_pda,
                derived_c_pda,
                pda_account.key.log(),
                address_space.log(),
            );
            return Err(LightSdkError::ConstraintViolation);
        }

        let space = compressed_account.size()?;
        let rent_minimum_balance = rent.minimum_balance(space + 100); // FIXME: use correct size

        // Create PDA account
        let create_account_ix = system_instruction::create_account(
            rent_payer.key,
            pda_account.key,
            rent_minimum_balance,
            space as u64,
            owner_program,
        );

        invoke_signed(
            &create_account_ix,
            &[
                rent_payer.clone(),
                (*pda_account).clone(),
                cpi_accounts.system_program()?.clone(),
            ],
            &[seeds],
        )?;

        // Initialize PDA with decompressed data and update slot
        let mut decompressed_pda = compressed_account.account.clone();
        decompressed_pda
            .compression_info_mut()
            .set_last_written_slot_value(current_slot);

        // set state to decompressed
        decompressed_pda.compression_info_mut().set_decompressed();

        #[cfg(feature = "anchor")]
        pda_account.try_borrow_mut_data()?[..8].copy_from_slice(&T::DISCRIMINATOR);
        #[cfg(not(feature = "anchor"))]
        pda_account.try_borrow_mut_data()?[..8].copy_from_slice(&T::discriminator());

        // Write data to PDA (after discriminator)
        // TODO: review this!
        decompressed_pda
            .serialize(&mut &mut pda_account.try_borrow_mut_data()?[8..])
            .map_err(|_| LightSdkError::Borsh)?;

        // Zero the compressed account data
        compressed_account.remove_data();
        // Add to CPI batch
        compressed_accounts_for_cpi.push(compressed_account.to_account_info()?);
    }

    // apply compressed account changes via cpi
    if !compressed_accounts_for_cpi.is_empty() {
        let cpi_inputs = CpiInputs::new(proof, compressed_accounts_for_cpi);
        cpi_inputs.invoke_light_system_program(cpi_accounts)?;
    }

    Ok(())
}
