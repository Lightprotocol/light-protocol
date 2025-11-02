#![allow(clippy::all)] // TODO: Remove.
#[cfg(feature = "anchor")]
use anchor_lang::Key;
#[allow(unused_imports)] // TODO: Remove.
#[cfg(feature = "anchor")]
use anchor_lang::{
    AccountsClose,
    {prelude::Account, AccountDeserialize, AccountSerialize},
};
#[cfg(feature = "anchor")]
use light_compressed_account::instruction_data::data::NewAddressParamsAssignedPacked;
use light_hasher::DataHasher;
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_pubkey::Pubkey;

use crate::{
    account::sha::LightAccount,
    compressible::HasCompressionInfo,
    cpi::{InvokeLightSystemProgram, LightCpiInstruction},
    error::{LightSdkError, Result},
    instruction::ValidityProof,
    AnchorDeserialize, AnchorSerialize, LightDiscriminator,
};

#[cfg(feature = "v2")]
use crate::cpi::v2::{CpiAccounts, LightSystemProgramCpi};

/// Wrapper to init an Anchor account as compressible and directly compress it.
/// Close the source PDA account manually at the end of the caller program's
/// init instruction.
#[cfg(feature = "anchor")]
pub fn compress_account_on_init<'info, A>(
    solana_account: &Account<'info, A>,
    address: &[u8; 32],
    new_address_param: &NewAddressParamsAssignedPacked,
    output_state_tree_index: u8,
    cpi_accounts: CpiAccounts<'_, 'info>,
    proof: ValidityProof,
) -> Result<()>
where
    A: DataHasher
        + LightDiscriminator
        + AnchorSerialize
        + AnchorDeserialize
        + AccountSerialize
        + AccountDeserialize
        + Default
        + Clone
        + HasCompressionInfo,
    A: std::fmt::Debug,
{
    let compressed_infos = prepare_accounts_for_compression_on_init(
        std::slice::from_ref(&solana_account),
        std::slice::from_ref(address),
        std::slice::from_ref(new_address_param),
        std::slice::from_ref(&output_state_tree_index),
        &cpi_accounts,
    )?;

    LightSystemProgramCpi::new_cpi(cpi_accounts.config().cpi_signer, proof)
        .with_new_addresses(&[*new_address_param])
        .with_account_infos(&compressed_infos)
        .invoke(cpi_accounts)?;

    Ok(())
}

/// Helper function to initialize a multiple Anchor accounts as compressible.
/// Returns account_infos so that all compressible accounts can be compressed in
/// a single CPI at the end of the caller program's init instruction.
///
/// # Arguments
/// * `solana_accounts` - The Anchor accounts to compress
/// * `addresses` - The addresses for the compressed accounts
/// * `new_address_params` - Address parameters for the compressed accounts
/// * `output_state_tree_indices` - Output state tree indices for the compressed
///   accounts
/// * `cpi_accounts` - Accounts needed for validation
///
/// # Returns
/// * `Ok(Vec<CompressedAccountInfo>)` - CompressedAccountInfo for CPI batching
/// * `Err(LightSdkError)` if there was an error
#[cfg(all(feature = "anchor", feature = "v2"))]
pub fn prepare_accounts_for_compression_on_init<'info, A>(
    solana_accounts: &[&Account<'info, A>],
    addresses: &[[u8; 32]],
    new_address_params: &[NewAddressParamsAssignedPacked],
    output_state_tree_indices: &[u8],
    cpi_accounts: &CpiAccounts<'_, 'info>,
) -> Result<Vec<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>>
where
    A: DataHasher
        + LightDiscriminator
        + AnchorSerialize
        + AnchorDeserialize
        + AccountSerialize
        + AccountDeserialize
        + Default
        + Clone
        + HasCompressionInfo,
    A: std::fmt::Debug,
{
    if solana_accounts.len() != addresses.len()
        || solana_accounts.len() != new_address_params.len()
        || solana_accounts.len() != output_state_tree_indices.len()
    {
        msg!(
            "Array length mismatch in prepare_accounts_for_compression_on_init - solana_accounts: {}, addresses: {}, new_address_params: {}, output_state_tree_indices: {}",
            solana_accounts.len(),
            addresses.len(),
            new_address_params.len(),
            output_state_tree_indices.len()
        );
        return Err(LightSdkError::ConstraintViolation);
    }

    let mut compressed_account_infos = Vec::new();

    for (((solana_account, &address), &_new_address_param), &output_state_tree_index) in
        solana_accounts
            .iter()
            .zip(addresses.iter())
            .zip(new_address_params.iter())
            .zip(output_state_tree_indices.iter())
    {
        // TODO: check security of not setting compressed so we don't need to pass as mut.
        // Ensure the account is marked as compressed We need to init first
        // because it's none. Setting to compressed prevents lamports funding
        // attack.
        // *solana_account.compression_info_mut_opt() =
        //     Some(super::CompressionInfo::new_decompressed()?);
        // solana_account.compression_info_mut().set_compressed();

        let owner_program_id = cpi_accounts.self_program_id();

        let mut compressed_account =
            LightAccount::<A>::new_init(&owner_program_id, Some(address), output_state_tree_index);

        // Clone the PDA data and set compression_info to None.
        let mut compressed_data = (***solana_account).clone();
        compressed_data.set_compression_info_none();
        compressed_account.account = compressed_data;

        compressed_account_infos.push(compressed_account.to_account_info()?);
    }

    Ok(compressed_account_infos)
}

/// Wrapper to process a single onchain PDA for creating an empty compressed
/// account.
///
/// The PDA account is NOT closed.
#[cfg(feature = "anchor")]
#[allow(clippy::too_many_arguments)]
pub fn compress_empty_account_on_init<'info, A>(
    solana_account: &mut Account<'info, A>,
    address: &[u8; 32],
    new_address_param: &NewAddressParamsAssignedPacked,
    output_state_tree_index: u8,
    cpi_accounts: CpiAccounts<'_, 'info>,
    proof: ValidityProof,
) -> Result<()>
where
    A: DataHasher
        + LightDiscriminator
        + AnchorSerialize
        + AnchorDeserialize
        + AccountSerialize
        + AccountDeserialize
        + Default
        + Clone
        + HasCompressionInfo,
{
    let compressed_infos = prepare_empty_compressed_accounts_on_init(
        &mut [solana_account],
        std::slice::from_ref(address),
        std::slice::from_ref(new_address_param),
        std::slice::from_ref(&output_state_tree_index),
        &cpi_accounts,
    )?;

    LightSystemProgramCpi::new_cpi(cpi_accounts.config().cpi_signer, proof)
        .with_new_addresses(&[*new_address_param])
        .with_account_infos(&compressed_infos)
        .invoke(cpi_accounts)?;

    Ok(())
}

/// Helper function to initialize multiple empty compressed PDA based on the
/// Anchor accounts addresses.
///
/// Use this over `prepare_accounts_for_compression_on_init` if you want to
/// initialize your Anchor accounts as compressible **without** compressing them
/// atomically.
///
/// # Arguments
/// * `solana_accounts` - The Anchor accounts
/// * `addresses` - The addresses for the compressed accounts
/// * `new_address_params` - Address parameters for the compressed accounts
/// * `output_state_tree_indices` - Output state tree indices for the compressed
///   accounts
/// * `cpi_accounts` - Accounts needed for validation
///
/// # Returns
/// * `Ok(Vec<CompressedAccountInfo>)` - CompressedAccountInfo for CPI batching
/// * `Err(LightSdkError)` if there was an error
#[cfg(all(feature = "anchor", feature = "v2"))]
pub fn prepare_empty_compressed_accounts_on_init<'info, A>(
    solana_accounts: &mut [&mut Account<'info, A>],
    addresses: &[[u8; 32]],
    new_address_params: &[NewAddressParamsAssignedPacked],
    output_state_tree_indices: &[u8],
    cpi_accounts: &CpiAccounts<'_, 'info>,
) -> Result<Vec<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>>
where
    A: DataHasher
        + LightDiscriminator
        + AnchorSerialize
        + AnchorDeserialize
        + AccountSerialize
        + AccountDeserialize
        + Default
        + Clone
        + HasCompressionInfo,
{
    if solana_accounts.len() != addresses.len()
        || solana_accounts.len() != new_address_params.len()
        || solana_accounts.len() != output_state_tree_indices.len()
    {
        msg!(
            "Array length mismatch in prepare_empty_compressed_accounts_on_init - solana_accounts: {}, addresses: {}, new_address_params: {}, output_state_tree_indices: {}",
            solana_accounts.len(),
            addresses.len(),
            new_address_params.len(),
            output_state_tree_indices.len()
        );
        return Err(LightSdkError::ConstraintViolation);
    }

    let mut compressed_account_infos = Vec::new();

    for (((solana_account, &address), &_new_address_param), &output_state_tree_index) in
        solana_accounts
            .iter_mut()
            .zip(addresses.iter())
            .zip(new_address_params.iter())
            .zip(output_state_tree_indices.iter())
    {
        // TODO: check security of not setting compressed so we don't need to pass as mut.
        // Ensure the account is marked as compressed We need to init first
        // because it's none. Setting to compressed prevents lamports funding
        // attack.
        *solana_account.compression_info_mut_opt() =
            Some(super::CompressionInfo::new_decompressed()?);

        let owner_program_id = cpi_accounts.self_program_id();

        // Create an empty compressed account with the specified address
        let mut compressed_account =
            LightAccount::<A>::new_init(&owner_program_id, Some(address), output_state_tree_index);

        compressed_account.remove_data();
        compressed_account_infos.push(compressed_account.to_account_info()?);
    }

    Ok(compressed_account_infos)
}
