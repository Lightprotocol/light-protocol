#![allow(clippy::all)] // TODO: Remove.

use light_compressed_account::{
    address::derive_address, instruction_data::with_account_info::CompressedAccountInfo,
};
use light_hasher::DataHasher;
use solana_account_info::AccountInfo;
use solana_cpi::invoke_signed;
use solana_msg::msg;
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use solana_system_interface::instruction as system_instruction;
use solana_sysvar::Sysvar;

use crate::{
    account::sha::LightAccount, compressible::compression_info::HasCompressionInfo,
    cpi::CpiAccountsSmall, error::LightSdkError, AnchorDeserialize, AnchorSerialize,
    LightDiscriminator,
};

/// Helper to invoke create_account with minimal stack usage
#[inline(never)]
#[cold]
fn invoke_create_account_heap<'info>(
    rent_payer: &AccountInfo<'info>,
    solana_account: &AccountInfo<'info>,
    rent_minimum_balance: u64,
    space: u64,
    program_id: &Pubkey,
    seeds: &[&[u8]],
    system_program: &AccountInfo<'info>,
) -> Result<(), LightSdkError> {
    // Box the instruction to reduce stack usage
    let create_account_ix = Box::new(system_instruction::create_account(
        rent_payer.key,
        solana_account.key,
        rent_minimum_balance,
        space,
        program_id,
    ));

    // Pre-allocate accounts on heap
    let accounts = Box::new(vec![
        rent_payer.clone(),
        solana_account.clone(),
        system_program.clone(),
    ]);

    invoke_signed(&*create_account_ix, &accounts[..], &[seeds])
        .map_err(|e| LightSdkError::ProgramError(e))
}

/// Helper function to process a single compressed account into PDA
/// This is a stack-safe version that processes one account at a time
/// Uses heap allocation for large data structures to minimize stack usage
#[inline(never)]
fn process_single_account<'info, T>(
    solana_account: &AccountInfo<'info>,
    compressed_account: LightAccount<'_, T>,
    seeds: &[&[u8]],
    cpi_accounts: &Box<CpiAccountsSmall<'_, 'info>>,
    rent_payer: &AccountInfo<'info>,
    address_space: Pubkey,
) -> Result<Option<CompressedAccountInfo>, LightSdkError>
where
    T: DataHasher
        + LightDiscriminator
        + AnchorSerialize
        + AnchorDeserialize
        + Default
        + Clone
        + HasCompressionInfo
        + crate::account::Size,
{
    // Check if PDA is already initialized
    if !solana_account.data_is_empty() {
        msg!("PDA already initialized, skipping");
        return Ok(None);
    }

    let rent = Rent::get().map_err(|_| LightSdkError::Borsh)?;
    let mut compressed_account = compressed_account; // Take ownership

    // Get the compressed account address
    let c_pda = compressed_account
        .address()
        .ok_or(LightSdkError::ConstraintViolation)?;

    // Box the address bytes to reduce stack usage during derivation
    let solana_key_bytes = Box::new(solana_account.key.to_bytes());
    let address_space_bytes = Box::new(address_space.to_bytes());
    let program_id_bytes = Box::new(cpi_accounts.self_program_id().to_bytes());

    let derived_c_pda = derive_address(
        &*solana_key_bytes,
        &*address_space_bytes,
        &*program_id_bytes,
    );

    // CHECK: pda and c_pda are related
    if c_pda != derived_c_pda {
        msg!("cPDA mismatch: {:?} != {:?}", c_pda, derived_c_pda);
        return Err(LightSdkError::ConstraintViolation);
    }

    let space = T::size(&compressed_account.account);

    let rent_minimum_balance = rent.minimum_balance(space);

    // Use the heap-optimized helper function
    let program_id = Box::new(cpi_accounts.self_program_id());
    invoke_create_account_heap(
        rent_payer,
        solana_account,
        rent_minimum_balance,
        space as u64,
        &*program_id,
        seeds,
        cpi_accounts.system_program()?,
    )?;

    // Initialize PDA with decompressed data
    let mut decompressed_pda = Box::new(compressed_account.account.clone());
    *decompressed_pda.compression_info_mut_opt() =
        Some(super::CompressionInfo::new_decompressed()?);

    // Copy discriminator
    let discriminator_len = T::LIGHT_DISCRIMINATOR.len();
    solana_account.try_borrow_mut_data()?[..discriminator_len]
        .copy_from_slice(&T::LIGHT_DISCRIMINATOR);

    // Serialize account data directly to the account's data buffer
    decompressed_pda
        .serialize(&mut &mut solana_account.try_borrow_mut_data()?[discriminator_len..])
        .map_err(|err| {
            msg!("Failed to serialize decompressed PDA: {:?}", err);
            LightSdkError::Borsh
        })?;

    compressed_account.remove_data();
    Ok(Some(compressed_account.to_account_info()?))
}

/// Helper function to decompress multiple compressed accounts into PDAs
/// idempotently with seeds. Does not invoke the zk compression CPI. This
/// function processes accounts of a single type and returns
/// CompressedAccountInfo for CPI batching. It's idempotent, meaning it can be
/// called multiple times with the same compressed accounts and it will only
/// decompress them once. If a PDA already exists and is initialized, it skips
/// that account.
///
/// # Arguments
/// * `solana_accounts` - The PDA accounts to decompress into
/// * `compressed_accounts` - The compressed accounts to decompress
/// * `solana_accounts_signer_seeds` - Signer seeds for each PDA including bump (standard Solana
///   format)
/// * `cpi_accounts` - Accounts needed for CPI
/// * `rent_payer` - The account to pay for PDA rent
/// * `address_space` - The address space for the compressed accounts
///
/// # Returns
/// * `Ok(Vec<CompressedAccountInfo>)` - CompressedAccountInfo for CPI batching
/// * `Err(LightSdkError)` if there was an error
#[inline(never)]
pub fn prepare_accounts_for_decompress_idempotent<'info, T>(
    solana_accounts: &Box<Vec<&AccountInfo<'info>>>,
    compressed_accounts: Box<Vec<LightAccount<'_, T>>>,
    solana_accounts_signer_seeds: &Box<Vec<&[&[u8]]>>,
    cpi_accounts: &Box<CpiAccountsSmall<'_, 'info>>,
    rent_payer: &AccountInfo<'info>,
    address_space: Pubkey,
) -> Result<Box<Vec<CompressedAccountInfo>>, LightSdkError>
where
    T: DataHasher
        + LightDiscriminator
        + AnchorSerialize
        + AnchorDeserialize
        + Default
        + Clone
        + HasCompressionInfo
        + crate::account::Size,
{
    // Execute processing on heap using closure
    (move || -> Result<Box<Vec<CompressedAccountInfo>>, LightSdkError> {
        // Validate input lengths

        if solana_accounts.len() != compressed_accounts.len()
            || solana_accounts.len() != solana_accounts_signer_seeds.len()
        {
            return Err(LightSdkError::ConstraintViolation);
        }

        let mut results = Box::new(Vec::new());

        // Process accounts using simple indexing to avoid iterator stack overhead
        let account_count = solana_accounts.len();

        // Convert to mutable for removing elements - unbox first
        let mut compressed_accounts = *compressed_accounts;

        for idx in 0..account_count {
            // Get account references directly without complex iterators
            let solana_account = solana_accounts[idx];
            // Take ownership by removing from vec (always remove first element as we process)
            let compressed_account = compressed_accounts.remove(0);
            let signer_seeds = solana_accounts_signer_seeds[idx];

            if let Some(compressed_info) = process_single_account(
                solana_account,
                compressed_account,
                signer_seeds,
                cpi_accounts,
                rent_payer,
                address_space,
            )? {
                results.push(compressed_info);
            }
        }

        Ok(results)
    })()
}
