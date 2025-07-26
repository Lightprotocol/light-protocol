#[cfg(feature = "anchor")]
use anchor_lang::AccountsClose;
#[cfg(feature = "anchor")]
use anchor_lang::{prelude::Account, AccountDeserialize, AccountSerialize};
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize as BorshDeserialize, AnchorSerialize as BorshSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::DataHasher;
use solana_account_info::AccountInfo;

use solana_pubkey::Pubkey;

use crate::{
    account::LightAccount,
    address::PackedNewAddressParams,
    compressible::HasCompressionInfo,
    cpi::{CpiAccounts, CpiInputs},
    error::LightSdkError,
    instruction::ValidityProof,
    light_account_checks::AccountInfoTrait,
    LightDiscriminator,
};

#[cfg(feature = "anchor")]
/// Wrapper to process a single onchain PDA for compression into a new
/// compressed account. Calls `process_accounts_for_compression_on_init` with
/// single-element slices and invokes the CPI.
#[allow(clippy::too_many_arguments)]
pub fn compress_account_on_init<'info, A>(
    pda_account: &mut Account<'info, A>,
    address: &[u8; 32],
    new_address_param: &PackedNewAddressParams,
    output_state_tree_index: u8,
    cpi_accounts: CpiAccounts<'_, 'info>,
    owner_program: &Pubkey,
    address_space: &[Pubkey],
    rent_recipient: &AccountInfo<'info>,
    proof: ValidityProof,
) -> Result<(), crate::ProgramError>
where
    A: DataHasher
        + LightDiscriminator
        + BorshSerialize
        + BorshDeserialize
        + Default
        + Clone
        + HasCompressionInfo
        + std::fmt::Debug,
    A: AccountSerialize + AccountDeserialize,
{
    let mut pda_accounts: [&mut Account<'info, A>; 1] = [pda_account];
    let addresses: [[u8; 32]; 1] = [*address];
    let new_address_params: [PackedNewAddressParams; 1] = [*new_address_param];
    let output_state_tree_indices: [u8; 1] = [output_state_tree_index];

    let compressed_infos = prepare_accounts_for_compression_on_init(
        &mut pda_accounts,
        &addresses,
        &new_address_params,
        &output_state_tree_indices,
        &cpi_accounts,
        owner_program,
        address_space,
        rent_recipient,
    )?;

    let cpi_inputs = CpiInputs::new_with_address(proof, compressed_infos, vec![*new_address_param]);

    cpi_inputs.invoke_light_system_program(cpi_accounts)?;

    Ok(())
}

#[cfg(feature = "anchor")]
/// Helper function to process multiple onchain PDAs for compression into new
/// compressed accounts.
///
/// This function processes accounts of a single type and returns
/// CompressedAccountInfo for CPI batching. It allows the caller to handle the
/// CPI invocation separately, enabling batching of multiple different account
/// types.
///
/// # Arguments
/// * `pda_accounts` - The PDA accounts to compress
/// * `addresses` - The addresses for the compressed accounts
/// * `new_address_params` - Address parameters for the compressed accounts
/// * `output_state_tree_indices` - Output state tree indices for the compressed
///   accounts
/// * `cpi_accounts` - Accounts needed for validation
/// * `owner_program` - The program that will own the compressed accounts
/// * `address_space` - The address space to validate uniqueness against
///
/// # Returns
/// * `Ok(Vec<CompressedAccountInfo>)` - CompressedAccountInfo for CPI batching
/// * `Err(LightSdkError)` if there was an error
#[allow(clippy::too_many_arguments)]
pub fn prepare_accounts_for_compression_on_init<'info, A>(
    pda_accounts: &mut [&mut Account<'info, A>],
    addresses: &[[u8; 32]],
    new_address_params: &[PackedNewAddressParams],
    output_state_tree_indices: &[u8],
    cpi_accounts: &CpiAccounts<'_, 'info>,
    owner_program: &Pubkey,
    address_space: &[Pubkey],
    rent_recipient: &AccountInfo<'info>,
) -> Result<
    Vec<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>,
    crate::ProgramError,
>
where
    A: DataHasher
        + LightDiscriminator
        + BorshSerialize
        + BorshDeserialize
        + Default
        + Clone
        + HasCompressionInfo
        + std::fmt::Debug,
    A: AccountSerialize + AccountDeserialize,
{
    if pda_accounts.len() != addresses.len()
        || pda_accounts.len() != new_address_params.len()
        || pda_accounts.len() != output_state_tree_indices.len()
    {
        return Err(LightSdkError::ConstraintViolation.into());
    }

    // Address space validation
    for params in new_address_params {
        let tree = cpi_accounts
            .get_tree_account_info(params.address_merkle_tree_account_index as usize)
            .map_err(|_| LightSdkError::ConstraintViolation)?
            .pubkey();
        if !address_space.iter().any(|a| a == &tree) {
            return Err(LightSdkError::ConstraintViolation.into());
        }
    }

    let mut compressed_account_infos = Vec::new();

    for (((pda_account, &address), &_new_address_param), &output_state_tree_index) in pda_accounts
        .iter_mut()
        .zip(addresses.iter())
        .zip(new_address_params.iter())
        .zip(output_state_tree_indices.iter())
    {
        // Ensure the account is marked as compressed We need to init first
        // because it's none. Setting to compressed prevents lamports funding
        // attack.
        *pda_account.compression_info_mut_opt() = Some(super::CompressionInfo::new()?);
        pda_account.compression_info_mut().set_compressed();

        // Create the compressed account with the PDA data
        let mut compressed_account =
            LightAccount::<'_, A>::new_init(owner_program, Some(address), output_state_tree_index);

        // Clone the PDA data and set compression_info to None for compressed
        // storage
        let mut compressed_data = (***pda_account).clone();
        compressed_data.set_compression_info_none();
        compressed_account.account = compressed_data;

        compressed_account_infos.push(compressed_account.to_account_info()?);

        // Close both PDA accounts
        pda_account.close(rent_recipient.clone())?;
    }

    Ok(compressed_account_infos)
}

/// Native Solana variant of compress_account_on_init that works with AccountInfo and pre-deserialized data.
/// 
/// Wrapper to process a single onchain PDA for compression into a new
/// compressed account. Calls `prepare_accounts_for_compression_on_init_native` with
/// single-element slices and invokes the CPI.
#[allow(clippy::too_many_arguments)]
pub fn compress_account_on_init_native<'info, A>(
    pda_account_info: &AccountInfo<'info>,
    pda_account_data: &mut A,
    address: &[u8; 32],
    new_address_param: &PackedNewAddressParams,
    output_state_tree_index: u8,
    cpi_accounts: CpiAccounts<'_, 'info>,
    owner_program: &Pubkey,
    address_space: &[Pubkey],
    rent_recipient: &AccountInfo<'info>,
    proof: ValidityProof,
) -> Result<(), crate::ProgramError>
where
    A: DataHasher
        + LightDiscriminator
        + BorshSerialize
        + BorshDeserialize
        + Default
        + Clone
        + HasCompressionInfo
        + std::fmt::Debug,
{
    let pda_accounts_info: [&AccountInfo<'info>; 1] = [pda_account_info];
    let mut pda_accounts_data: [&mut A; 1] = [pda_account_data];
    let addresses: [[u8; 32]; 1] = [*address];
    let new_address_params: [PackedNewAddressParams; 1] = [*new_address_param];
    let output_state_tree_indices: [u8; 1] = [output_state_tree_index];

    let compressed_infos = prepare_accounts_for_compression_on_init_native(
        &pda_accounts_info,
        &mut pda_accounts_data,
        &addresses,
        &new_address_params,
        &output_state_tree_indices,
        &cpi_accounts,
        owner_program,
        address_space,
        rent_recipient,
    )?;

    let cpi_inputs = CpiInputs::new_with_address(proof, compressed_infos, vec![*new_address_param]);

    cpi_inputs.invoke_light_system_program(cpi_accounts)?;

    Ok(())
}

/// Native Solana variant of prepare_accounts_for_compression_on_init that works with AccountInfo and pre-deserialized data.
/// 
/// Helper function to process multiple onchain PDAs for compression into new
/// compressed accounts.
///
/// This function processes accounts of a single type and returns
/// CompressedAccountInfo for CPI batching. It allows the caller to handle the
/// CPI invocation separately, enabling batching of multiple different account
/// types.
///
/// # Arguments
/// * `pda_accounts_info` - The PDA AccountInfos to compress
/// * `pda_accounts_data` - The pre-deserialized PDA account data
/// * `addresses` - The addresses for the compressed accounts
/// * `new_address_params` - Address parameters for the compressed accounts
/// * `output_state_tree_indices` - Output state tree indices for the compressed
///   accounts
/// * `cpi_accounts` - Accounts needed for validation
/// * `owner_program` - The program that will own the compressed accounts
/// * `address_space` - The address space to validate uniqueness against
/// * `rent_recipient` - The account to receive the PDAs' rent
///
/// # Returns
/// * `Ok(Vec<CompressedAccountInfo>)` - CompressedAccountInfo for CPI batching
/// * `Err(LightSdkError)` if there was an error
#[allow(clippy::too_many_arguments)]
pub fn prepare_accounts_for_compression_on_init_native<'info, A>(
    pda_accounts_info: &[&AccountInfo<'info>],
    pda_accounts_data: &mut [&mut A],
    addresses: &[[u8; 32]],
    new_address_params: &[PackedNewAddressParams],
    output_state_tree_indices: &[u8],
    cpi_accounts: &CpiAccounts<'_, 'info>,
    owner_program: &Pubkey,
    address_space: &[Pubkey],
    rent_recipient: &AccountInfo<'info>,
) -> Result<
    Vec<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>,
    crate::ProgramError,
>
where
    A: DataHasher
        + LightDiscriminator
        + BorshSerialize
        + BorshDeserialize
        + Default
        + Clone
        + HasCompressionInfo
        + std::fmt::Debug,
{
    if pda_accounts_info.len() != pda_accounts_data.len()
        || pda_accounts_info.len() != addresses.len()
        || pda_accounts_info.len() != new_address_params.len()
        || pda_accounts_info.len() != output_state_tree_indices.len()
    {
        return Err(LightSdkError::ConstraintViolation.into());
    }

    // Address space validation
    for params in new_address_params {
        let tree = cpi_accounts
            .get_tree_account_info(params.address_merkle_tree_account_index as usize)
            .map_err(|_| LightSdkError::ConstraintViolation)?
            .pubkey();
        if !address_space.iter().any(|a| a == &tree) {
            return Err(LightSdkError::ConstraintViolation.into());
        }
    }

    let mut compressed_account_infos = Vec::new();

    for ((((pda_account_info, pda_account_data), &address), &_new_address_param), &output_state_tree_index) in pda_accounts_info
        .iter()
        .zip(pda_accounts_data.iter_mut())
        .zip(addresses.iter())
        .zip(new_address_params.iter())
        .zip(output_state_tree_indices.iter())
    {
        // Ensure the account is marked as compressed We need to init first
        // because it's none. Setting to compressed prevents lamports funding
        // attack.
        *pda_account_data.compression_info_mut_opt() = Some(super::CompressionInfo::new()?);
        pda_account_data.compression_info_mut().set_compressed();

        // Create the compressed account with the PDA data
        let mut compressed_account =
            LightAccount::<'_, A>::new_init(owner_program, Some(address), output_state_tree_index);

        // Clone the PDA data and set compression_info to None for compressed
        // storage
        let mut compressed_data = (*pda_account_data).clone();
        compressed_data.set_compression_info_none();
        compressed_account.account = compressed_data;

        compressed_account_infos.push(compressed_account.to_account_info()?);

        // Close PDA account manually (native Solana way)
        let dest_starting_lamports = rent_recipient.lamports();
        **rent_recipient.try_borrow_mut_lamports()? = dest_starting_lamports
            .checked_add(pda_account_info.lamports())
            .ok_or(LightSdkError::TransferIntegerOverflow)?;

        // Zero out the PDA account
        **pda_account_info.try_borrow_mut_lamports()? = 0;
        pda_account_info.try_borrow_mut_data()?.fill(0);
    }

    Ok(compressed_account_infos)
}
