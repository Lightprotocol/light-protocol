//! Native Solana helpers for compressing accounts on init. Anchor-free.

#![allow(clippy::all)] // TODO: Remove.
#[allow(unused_imports)] // TODO: Remove.
use light_compressed_account::instruction_data::data::NewAddressParamsAssignedPacked;
use light_hasher::DataHasher;
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_pubkey::Pubkey;

use crate::{
    account::sha::LightAccount,
    address::PackedNewAddressParams,
    compressible::HasCompressionInfo,
    cpi::{
        v2::{CpiAccounts, LightSystemProgramCpi},
        InvokeLightSystemProgram, LightCpiInstruction,
    },
    error::{LightSdkError, Result},
    instruction::ValidityProof,
    light_account_checks::AccountInfoTrait,
    AnchorDeserialize, AnchorSerialize, LightDiscriminator,
};

/// Native Solana variant of compress_account_on_init that works with raw AccountInfo and pre-deserialized data.
///
/// Wrapper to init an raw PDA as compressible and directly compress it.
/// Calls `prepare_accounts_for_compression_on_init_native` with single-element
/// slices and invokes the CPI. Close the source PDA account manually.
#[allow(clippy::too_many_arguments)]
pub fn compress_account_on_init_native<'info, A>(
    pda_account_info: &mut AccountInfo<'info>,
    pda_account_data: &mut A,
    address: &[u8; 32],
    new_address_param: &PackedNewAddressParams,
    output_state_tree_index: u8,
    cpi_accounts: CpiAccounts<'_, 'info>,
    address_space: &[Pubkey],
    rent_recipient: &AccountInfo<'info>,
    proof: ValidityProof,
) -> Result<()>
where
    A: DataHasher
        + LightDiscriminator
        + AnchorSerialize
        + AnchorDeserialize
        + Default
        + Clone
        + HasCompressionInfo,
{
    // let pda_accounts_info:  = &[pda_account_info];
    let mut pda_accounts_data: [&mut A; 1] = [pda_account_data];
    let addresses: [[u8; 32]; 1] = [*address];
    let new_address_params: [PackedNewAddressParams; 1] = [*new_address_param];
    let output_state_tree_indices: [u8; 1] = [output_state_tree_index];

    let compressed_infos = prepare_accounts_for_compression_on_init_native(
        &mut [pda_account_info],
        &mut pda_accounts_data,
        &addresses,
        &new_address_params,
        &output_state_tree_indices,
        &cpi_accounts,
        address_space,
        rent_recipient,
    )?;

    LightSystemProgramCpi::new_cpi(cpi_accounts.config().cpi_signer, proof)
        .with_new_addresses(&[
            light_compressed_account::instruction_data::data::NewAddressParamsAssignedPacked::new(
                *new_address_param,
                None,
            ),
        ])
        .with_account_infos(&compressed_infos)
        .invoke(cpi_accounts)?;

    Ok(())
}

/// Native Solana variant of prepare_accounts_for_compression_on_init that works
/// with AccountInfo and pre-deserialized data.
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
/// * `address_space` - The address space to validate uniqueness against
/// * `rent_recipient` - The account to receive the PDAs' rent
///
/// # Returns
/// * `Ok(Vec<CompressedAccountInfo>)` - CompressedAccountInfo for CPI batching
/// * `Err(LightSdkError)` if there was an error
#[allow(clippy::too_many_arguments)]
pub fn prepare_accounts_for_compression_on_init_native<'info, A>(
    pda_accounts_info: &mut [&mut AccountInfo<'info>],
    pda_accounts_data: &mut [&mut A],
    addresses: &[[u8; 32]],
    new_address_params: &[PackedNewAddressParams],
    output_state_tree_indices: &[u8],
    cpi_accounts: &CpiAccounts<'_, 'info>,
    address_space: &[Pubkey],
    rent_recipient: &AccountInfo<'info>,
) -> Result<Vec<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>>
where
    A: DataHasher
        + LightDiscriminator
        + AnchorSerialize
        + AnchorDeserialize
        + Default
        + Clone
        + HasCompressionInfo,
{
    if pda_accounts_info.len() != pda_accounts_data.len()
        || pda_accounts_info.len() != addresses.len()
        || pda_accounts_info.len() != new_address_params.len()
        || pda_accounts_info.len() != output_state_tree_indices.len()
    {
        msg!("pda_accounts_info.len(): {:?}", pda_accounts_info.len());
        msg!("pda_accounts_data.len(): {:?}", pda_accounts_data.len());
        msg!("addresses.len(): {:?}", addresses.len());
        msg!("new_address_params.len(): {:?}", new_address_params.len());
        msg!(
            "output_state_tree_indices.len(): {:?}",
            output_state_tree_indices.len()
        );
        return Err(LightSdkError::ConstraintViolation);
    }

    // Address space validation
    for params in new_address_params {
        let tree = cpi_accounts
            .get_tree_account_info(params.address_merkle_tree_account_index as usize)
            .map_err(|_| {
                msg!(
                    "Failed to get tree account info at index {} in prepare_accounts_for_compression_on_init_native",
                    params.address_merkle_tree_account_index
                );
                LightSdkError::ConstraintViolation
            })?
            .pubkey();
        if !address_space.iter().any(|a| a == &tree) {
            msg!("address tree: {:?}", tree);
            msg!("expected address_space: {:?}", address_space);
            msg!("Address tree {} not found in allowed address space in prepare_accounts_for_compression_on_init_native", tree);
            return Err(LightSdkError::ConstraintViolation);
        }
    }

    let mut compressed_account_infos = Vec::new();

    for (
        (((pda_account_info, pda_account_data), &address), &_new_address_param),
        &output_state_tree_index,
    ) in pda_accounts_info
        .iter_mut()
        .zip(pda_accounts_data.iter_mut())
        .zip(addresses.iter())
        .zip(new_address_params.iter())
        .zip(output_state_tree_indices.iter())
    {
        // Ensure the account is marked as compressed We need to init first
        // because it's none. Setting to compressed prevents lamports funding
        // attack.
        *pda_account_data.compression_info_mut_opt() =
            Some(super::CompressionInfo::new_decompressed()?);
        pda_account_data.compression_info_mut().set_compressed();

        // Create the compressed account with the PDA data
        let owner_program_id = cpi_accounts.self_program_id();
        let mut compressed_account = LightAccount::<'_, A>::new_init(
            &owner_program_id,
            Some(address),
            output_state_tree_index,
        );

        // Clone the PDA data and set compression_info to None for compressed
        // storage
        let mut compressed_data = (*pda_account_data).clone();
        compressed_data.set_compression_info_none();
        compressed_account.account = compressed_data;

        compressed_account_infos.push(compressed_account.to_account_info()?);

        // Close PDA account manually
        close(pda_account_info, rent_recipient.clone()).map_err(|err| {
            msg!("Failed to close PDA account in prepare_accounts_for_compression_on_init_native: {:?}", err);
            err
        })?;
    }

    Ok(compressed_account_infos)
}

/// Native Solana variant to create an EMPTY compressed account from a PDA.
///
/// This creates an empty compressed account without closing the source PDA,
/// similar to decompress_idempotent behavior. The PDA remains intact with its data.
///
/// # Arguments
/// * `pda_account_info` - The PDA AccountInfo (will NOT be closed)
/// * `pda_account_data` - The pre-deserialized PDA account data  
/// * `address` - The address for the compressed account
/// * `new_address_param` - Address parameters for the compressed account
/// * `output_state_tree_index` - Output state tree index for the compressed account
/// * `cpi_accounts` - Accounts needed for validation
/// * `address_space` - The address space to validate uniqueness against
/// * `proof` - Validity proof for the address tree operation
#[allow(clippy::too_many_arguments)]
pub fn compress_empty_account_on_init_native<'info, A>(
    pda_account_info: &mut AccountInfo<'info>,
    pda_account_data: &mut A,
    address: &[u8; 32],
    new_address_param: &PackedNewAddressParams,
    output_state_tree_index: u8,
    cpi_accounts: CpiAccounts<'_, 'info>,
    address_space: &[Pubkey],
    proof: ValidityProof,
) -> Result<()>
where
    A: DataHasher
        + LightDiscriminator
        + AnchorSerialize
        + AnchorDeserialize
        + Default
        + Clone
        + HasCompressionInfo,
{
    let mut pda_accounts_data: [&mut A; 1] = [pda_account_data];
    let addresses: [[u8; 32]; 1] = [*address];
    let new_address_params: [PackedNewAddressParams; 1] = [*new_address_param];
    let output_state_tree_indices: [u8; 1] = [output_state_tree_index];

    let compressed_infos = prepare_empty_compressed_accounts_on_init_native(
        &mut [pda_account_info],
        &mut pda_accounts_data,
        &addresses,
        &new_address_params,
        &output_state_tree_indices,
        &cpi_accounts,
        address_space,
    )?;

    LightSystemProgramCpi::new_cpi(cpi_accounts.config().cpi_signer, proof)
        .with_new_addresses(&[
            light_compressed_account::instruction_data::data::NewAddressParamsAssignedPacked::new(
                *new_address_param,
                None,
            ),
        ])
        .with_account_infos(&compressed_infos)
        .invoke(cpi_accounts)?;

    Ok(())
}

/// Native Solana variant to create EMPTY compressed accounts from PDAs.
///
/// This creates empty compressed accounts without closing the source PDAs.
/// The PDAs remain intact with their data, similar to decompress_idempotent behavior.
///
/// # Arguments
/// * `pda_accounts_info` - The PDA AccountInfos (will NOT be closed)
/// * `pda_accounts_data` - The pre-deserialized PDA account data
/// * `addresses` - The addresses for the compressed accounts
/// * `new_address_params` - Address parameters for the compressed accounts
/// * `output_state_tree_indices` - Output state tree indices for the compressed accounts
/// * `cpi_accounts` - Accounts needed for validation
/// * `address_space` - The address space to validate uniqueness against
///
/// # Returns
/// * `Ok(Vec<CompressedAccountInfo>)` - CompressedAccountInfo for CPI batching
/// * `Err(LightSdkError)` if there was an error
#[allow(clippy::too_many_arguments)]
pub fn prepare_empty_compressed_accounts_on_init_native<'info, A>(
    _pda_accounts_info: &mut [&mut AccountInfo<'info>],
    pda_accounts_data: &mut [&mut A],
    addresses: &[[u8; 32]],
    new_address_params: &[PackedNewAddressParams],
    output_state_tree_indices: &[u8],
    cpi_accounts: &CpiAccounts<'_, 'info>,
    address_space: &[Pubkey],
) -> Result<Vec<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>>
where
    A: DataHasher
        + LightDiscriminator
        + AnchorSerialize
        + AnchorDeserialize
        + Default
        + Clone
        + HasCompressionInfo,
{
    if pda_accounts_data.len() != addresses.len()
        || pda_accounts_data.len() != new_address_params.len()
        || pda_accounts_data.len() != output_state_tree_indices.len()
    {
        msg!("pda_accounts_data.len(): {:?}", pda_accounts_data.len());
        msg!("addresses.len(): {:?}", addresses.len());
        msg!("new_address_params.len(): {:?}", new_address_params.len());
        msg!(
            "output_state_tree_indices.len(): {:?}",
            output_state_tree_indices.len()
        );
        return Err(LightSdkError::ConstraintViolation);
    }

    // Address space validation
    for params in new_address_params {
        let tree = cpi_accounts
            .get_tree_account_info(params.address_merkle_tree_account_index as usize)
            .map_err(|_| {
                msg!(
                    "Failed to get tree account info at index {} in prepare_empty_compressed_accounts_on_init_native",
                    params.address_merkle_tree_account_index
                );
                LightSdkError::ConstraintViolation
            })?
            .pubkey();
        if !address_space.iter().any(|a| a == &tree) {
            msg!("address tree: {:?}", tree);
            msg!("expected address_space: {:?}", address_space);
            return Err(LightSdkError::ConstraintViolation);
        }
    }

    let mut compressed_account_infos = Vec::new();

    for (((pda_account_data, &address), &_new_address_param), &output_state_tree_index) in
        pda_accounts_data
            .iter_mut()
            .zip(addresses.iter())
            .zip(new_address_params.iter())
            .zip(output_state_tree_indices.iter())
    {
        *pda_account_data.compression_info_mut_opt() =
            Some(super::CompressionInfo::new_decompressed()?);
        pda_account_data
            .compression_info_mut()
            .bump_last_written_slot()?;

        let owner_program_id = cpi_accounts.self_program_id();
        let mut light_account = LightAccount::<'_, A>::new_init(
            &owner_program_id,
            Some(address),
            output_state_tree_index,
        );
        light_account.remove_data();

        compressed_account_infos.push(light_account.to_account_info()?);
    }

    Ok(compressed_account_infos)
}

// Proper native Solana account closing implementation
pub fn close<'info>(
    info: &mut AccountInfo<'info>,
    sol_destination: AccountInfo<'info>,
) -> Result<()> {
    // Transfer all lamports from the account to the destination
    let lamports_to_transfer = info.lamports();

    // Use try_borrow_mut_lamports for proper borrow management
    **info
        .try_borrow_mut_lamports()
        .map_err(|_| LightSdkError::ConstraintViolation)? = 0;

    let dest_lamports = sol_destination.lamports();
    **sol_destination
        .try_borrow_mut_lamports()
        .map_err(|_| LightSdkError::ConstraintViolation)? =
        dest_lamports.checked_add(lamports_to_transfer).unwrap();

    // Assign to system program first
    let system_program_id = solana_pubkey::pubkey!("11111111111111111111111111111111");

    info.assign(&system_program_id);

    // Realloc to 0 size - this should work after assigning to system program
    info.realloc(0, false).map_err(|e| {
        msg!("Error during realloc: {:?}", e);
        LightSdkError::ConstraintViolation
    })?;

    Ok(())
}
