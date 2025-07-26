#[cfg(feature = "anchor")]
use anchor_lang::Discriminator as AnchorDiscriminatorShim;
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize as BorshDeserialize, AnchorSerialize as BorshSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::{
    address::derive_address, instruction_data::with_account_info::CompressedAccountInfo,
};
use light_hasher::DataHasher;
use solana_account_info::AccountInfo;
use solana_clock::Clock;
use solana_cpi::invoke_signed;
use solana_msg::msg;
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use solana_system_interface::instruction as system_instruction;
use solana_sysvar::Sysvar;

use crate::{
    account::LightAccount, compressible::compression_info::HasCompressionInfo, cpi::CpiAccounts,
    error::LightSdkError, LightDiscriminator,
};

#[cfg(not(feature = "anchor"))]
trait AnchorDiscriminatorShim {}
#[cfg(not(feature = "anchor"))]
impl<T> AnchorDiscriminatorShim for T {}

/// Helper function to decompress multiple compressed accounts into PDAs idempotently with seeds.
/// Does not invoke the zk compression CPI.
/// This function processes accounts of a single type and returns CompressedAccountInfo for CPI batching.
/// It's idempotent, meaning it can be called multiple times with the same compressed accounts
/// and it will only decompress them once. If a PDA already exists and is initialized, it skips that account.
///
/// # Arguments
/// * `pda_accounts` - The PDA accounts to decompress into
/// * `compressed_accounts` - The compressed accounts to decompress
/// * `signer_seeds` - Signer seeds for each PDA including bump (standard Solana format)
/// * `cpi_accounts` - Accounts needed for CPI
/// * `owner_program` - The program that will own the PDAs
/// * `rent_payer` - The account to pay for PDA rent
/// * `address_space` - The address space for the compressed accounts
///
/// # Returns
/// * `Ok(Vec<CompressedAccountInfo>)` - CompressedAccountInfo for CPI batching
/// * `Err(LightSdkError)` if there was an error
pub fn prepare_accounts_for_decompress_idempotent<'info, T>(
    pda_accounts: &[&AccountInfo<'info>],
    compressed_accounts: Vec<LightAccount<'_, T>>,
    signer_seeds: &[&[&[u8]]],
    cpi_accounts: &CpiAccounts<'_, 'info>,
    owner_program: &Pubkey,
    rent_payer: &AccountInfo<'info>,
    address_space: Pubkey,
) -> Result<Vec<CompressedAccountInfo>, LightSdkError>
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

        *decompressed_pda.compression_info_mut_opt() = Some(super::CompressionInfo::new()?);

        decompressed_pda
            .compression_info_mut()
            .set_last_written_slot_value(current_slot);

        // set state to decompressed
        decompressed_pda.compression_info_mut().set_decompressed();

        #[cfg(feature = "anchor")]
        pda_account.try_borrow_mut_data()?[..8].copy_from_slice(T::DISCRIMINATOR);
        // TODO: test without anchor
        #[cfg(not(feature = "anchor"))]
        pda_account.try_borrow_mut_data()?[..8].copy_from_slice(&T::discriminator());

        decompressed_pda
            .serialize(&mut &mut pda_account.try_borrow_mut_data()?[8..])
            .map_err(|_| LightSdkError::Borsh)?;

        compressed_account.remove_data();

        compressed_accounts_for_cpi.push(compressed_account.to_account_info()?);
    }

    Ok(compressed_accounts_for_cpi)
}
