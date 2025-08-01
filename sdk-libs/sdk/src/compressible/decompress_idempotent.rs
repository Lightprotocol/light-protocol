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
    cpi::CpiAccounts, error::LightSdkError, AnchorDeserialize, AnchorSerialize, LightDiscriminator,
};

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
pub fn prepare_accounts_for_decompress_idempotent<'info, T>(
    solana_accounts: &[&AccountInfo<'info>],
    compressed_accounts: Vec<LightAccount<'_, T>>,
    solana_accounts_signer_seeds: &[&[&[u8]]],
    cpi_accounts: &CpiAccounts<'_, 'info>,
    rent_payer: &AccountInfo<'info>,
    address_space: Pubkey,
) -> Result<Vec<CompressedAccountInfo>, LightSdkError>
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
    // Validate input lengths
    if solana_accounts.len() != compressed_accounts.len()
        || solana_accounts.len() != solana_accounts_signer_seeds.len()
    {
        return Err(LightSdkError::ConstraintViolation);
    }

    let rent = Rent::get().map_err(|_| LightSdkError::Borsh)?;

    let mut compressed_accounts_for_cpi = Vec::new();

    for ((solana_account, mut compressed_account), seeds) in solana_accounts
        .iter()
        .zip(compressed_accounts.into_iter())
        .zip(solana_accounts_signer_seeds.iter())
    {
        msg!("solana_account: {:?}", solana_account);
        // Check if PDA is already initialized
        if !solana_account.data_is_empty() {
            msg!(
                "PDA DATA {} already initialized, skipping decompression",
                solana_account.key
            );
            continue;
        }

        // Get the compressed account address
        let c_pda = compressed_account
            .address()
            .ok_or(LightSdkError::ConstraintViolation)?;

        let derived_c_pda = derive_address(
            &solana_account.key.to_bytes(),
            &address_space.to_bytes(),
            &cpi_accounts.self_program_id().to_bytes(),
        );

        // CHECK:
        // pda and c_pda are related
        if c_pda != derived_c_pda {
            msg!(
                "cPDA {:?} does not match derived cPDA {:?} for PDA {:?} with address space {:?}",
                c_pda,
                derived_c_pda,
                solana_account.key,
                address_space,
            );
            return Err(LightSdkError::ConstraintViolation);
        }

        let space = T::size(&compressed_account.account);
        let rent_minimum_balance = rent.minimum_balance(space);

        // Create PDA account
        let create_account_ix = system_instruction::create_account(
            rent_payer.key,
            solana_account.key,
            rent_minimum_balance,
            space as u64,
            &cpi_accounts.self_program_id(),
        );

        invoke_signed(
            &create_account_ix,
            &[
                rent_payer.clone(),
                (*solana_account).clone(),
                cpi_accounts.system_program()?.clone(),
            ],
            &[seeds],
        )?;

        // Initialize PDA with decompressed data and current slot
        let mut decompressed_pda = compressed_account.account.clone();
        *decompressed_pda.compression_info_mut_opt() =
            Some(super::CompressionInfo::new_decompressed()?);

        // This forces all programs to implement the LightDiscriminator trait but
        // since anchor 0.31.0 this can be any length.
        let discriminator_len = T::LIGHT_DISCRIMINATOR.len();
        solana_account.try_borrow_mut_data()?[..discriminator_len]
            .copy_from_slice(&T::LIGHT_DISCRIMINATOR);

        decompressed_pda
            .serialize(&mut &mut solana_account.try_borrow_mut_data()?[discriminator_len..])
            .map_err(|err| {
                msg!("Failed to serialize decompressed PDA: {:?}", err);
                LightSdkError::Borsh
            })?;

        compressed_account.remove_data();

        compressed_accounts_for_cpi.push(compressed_account.to_account_info()?);
    }

    Ok(compressed_accounts_for_cpi)
}
