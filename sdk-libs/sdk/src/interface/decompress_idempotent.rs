#![allow(clippy::all)] // TODO: Remove.

use light_compressed_account::{
    address::derive_address,
    instruction_data::with_account_info::{CompressedAccountInfo, OutAccountInfo},
};
use light_compressible::DECOMPRESSED_PDA_DISCRIMINATOR;
use light_hasher::{sha256::Sha256BE, Hasher};
use light_sdk_types::instruction::account_meta::{
    CompressedAccountMeta, CompressedAccountMetaNoLamportsNoAddress,
};
use solana_account_info::AccountInfo;
use solana_cpi::invoke_signed;
use solana_msg::msg;
use solana_pubkey::Pubkey;
use solana_system_interface::instruction as system_instruction;
use solana_sysvar::{rent::Rent, Sysvar};

use crate::{
    account::sha::LightAccount,
    compressible::compression_info::{CompressionInfo, HasCompressionInfo},
    cpi::v2::CpiAccounts,
    error::LightSdkError,
    AnchorDeserialize, AnchorSerialize, LightDiscriminator,
};

/// Set output for decompressed PDA format.
/// Isolated in separate function to reduce stack usage.
#[inline(never)]
#[cfg(feature = "v2")]
fn set_decompressed_pda_output(
    output: &mut OutAccountInfo,
    pda_pubkey_bytes: &[u8; 32],
) -> Result<(), LightSdkError> {
    output.data = pda_pubkey_bytes.to_vec();
    output.data_hash = Sha256BE::hash(pda_pubkey_bytes)?;
    output.discriminator = DECOMPRESSED_PDA_DISCRIMINATOR;
    Ok(())
}

/// Convert a `CompressedAccountMetaNoLamportsNoAddress` to a
/// `CompressedAccountMeta` by deriving the compressed address from the solana
/// account's pubkey.
pub fn into_compressed_meta_with_address<'info>(
    compressed_meta_no_lamports_no_address: &CompressedAccountMetaNoLamportsNoAddress,
    solana_account: &AccountInfo<'info>,
    address_space: Pubkey,
    program_id: &Pubkey,
) -> CompressedAccountMeta {
    let derived_c_pda = derive_address(
        &solana_account.key.to_bytes(),
        &address_space.to_bytes(),
        &program_id.to_bytes(),
    );

    let meta_with_address = CompressedAccountMeta {
        tree_info: compressed_meta_no_lamports_no_address.tree_info,
        address: derived_c_pda,
        output_state_tree_index: compressed_meta_no_lamports_no_address.output_state_tree_index,
    };

    meta_with_address
}

// TODO: consider folding into main fn.
/// Helper to invoke create_account on heap.
#[inline(never)]
fn invoke_create_account_with_heap<'info>(
    rent_sponsor: &AccountInfo<'info>,
    solana_account: &AccountInfo<'info>,
    rent_minimum_balance: u64,
    space: u64,
    program_id: &Pubkey,
    seeds: &[&[u8]],
    system_program: &AccountInfo<'info>,
) -> Result<(), LightSdkError> {
    let create_account_ix = system_instruction::create_account(
        rent_sponsor.key,
        solana_account.key,
        rent_minimum_balance,
        space,
        program_id,
    );

    invoke_signed(
        &create_account_ix,
        &[
            rent_sponsor.clone(),
            solana_account.clone(),
            system_program.clone(),
        ],
        &[seeds],
    )
    .map_err(|e| LightSdkError::ProgramError(e))
}

/// Maximum number of seeds for PDA derivation.
pub const MAX_SEEDS: usize = 16;

/// Convert Vec seeds to fixed array and call PDA creation.
/// Isolated in separate function to reduce stack usage (seed_refs array is on its own frame).
#[inline(never)]
#[cfg(feature = "v2")]
pub fn prepare_account_for_decompression_with_vec_seeds<'a, 'info, T>(
    program_id: &Pubkey,
    data: T,
    compressed_meta: CompressedAccountMeta,
    solana_account: &AccountInfo<'info>,
    rent_sponsor: &AccountInfo<'info>,
    cpi_accounts: &CpiAccounts<'a, 'info>,
    seeds_vec: &[Vec<u8>],
) -> Result<
    Option<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>,
    LightSdkError,
>
where
    T: Clone
        + crate::account::Size
        + LightDiscriminator
        + Default
        + AnchorSerialize
        + AnchorDeserialize
        + HasCompressionInfo
        + 'info,
{
    // Convert Vec seeds to fixed array on this stack frame
    let mut seed_refs: [&[u8]; MAX_SEEDS] = [&[]; MAX_SEEDS];
    let len = seeds_vec.len().min(MAX_SEEDS);
    for j in 0..len {
        seed_refs[j] = seeds_vec[j].as_slice();
    }

    prepare_account_for_decompression_idempotent(
        program_id,
        data,
        compressed_meta,
        solana_account,
        rent_sponsor,
        cpi_accounts,
        &seed_refs[..len],
    )
}

/// Helper function to decompress a compressed account into a PDA
/// idempotently with seeds.
#[inline(never)]
#[cfg(feature = "v2")]
pub fn prepare_account_for_decompression_idempotent<'a, 'info, T>(
    program_id: &Pubkey,
    data: T,
    compressed_meta: CompressedAccountMeta,
    solana_account: &AccountInfo<'info>,
    rent_sponsor: &AccountInfo<'info>,
    cpi_accounts: &CpiAccounts<'a, 'info>,
    signer_seeds: &[&[u8]],
) -> Result<
    Option<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>,
    LightSdkError,
>
where
    T: Clone
        + crate::account::Size
        + LightDiscriminator
        + Default
        + AnchorSerialize
        + AnchorDeserialize
        + HasCompressionInfo
        + 'info,
{
    if !solana_account.data_is_empty() {
        msg!("Account already initialized, skipping");
        return Ok(None);
    }
    let rent = Rent::get().map_err(|err| {
        msg!("Failed to get rent: {:?}", err);
        LightSdkError::Borsh
    })?;

    let light_account = LightAccount::<T>::new_close(program_id, &compressed_meta, data)?;

    // Account space needs to include discriminator + serialized data
    // T::size() already includes the full Option<CompressionInfo> footprint
    let discriminator_len = T::LIGHT_DISCRIMINATOR.len();
    let space = discriminator_len + T::size(&light_account.account)?;
    let rent_minimum_balance = rent.minimum_balance(space);

    invoke_create_account_with_heap(
        rent_sponsor,
        solana_account,
        rent_minimum_balance,
        space as u64,
        &cpi_accounts.self_program_id(),
        signer_seeds,
        cpi_accounts.system_program()?,
    )?;

    let mut decompressed_pda = light_account.account.clone();
    *decompressed_pda.compression_info_mut_opt() = Some(CompressionInfo::new_decompressed()?);

    let mut account_data = solana_account.try_borrow_mut_data()?;
    let discriminator_len = T::LIGHT_DISCRIMINATOR.len();
    account_data[..discriminator_len].copy_from_slice(&T::LIGHT_DISCRIMINATOR);
    decompressed_pda
        .serialize(&mut &mut account_data[discriminator_len..])
        .map_err(|err| {
            msg!("Failed to serialize decompressed PDA: {:?}", err);
            LightSdkError::Borsh
        })?;

    let mut account_info_result = light_account.to_account_info()?;

    // Set output to use decompressed PDA format:
    // - discriminator: DECOMPRESSED_PDA_DISCRIMINATOR
    // - data: PDA pubkey (32 bytes)
    // - data_hash: Sha256BE(pda_pubkey)
    if let Some(output) = account_info_result.output.as_mut() {
        set_decompressed_pda_output(output, &solana_account.key.to_bytes())?;
    }

    Ok(Some(account_info_result))
}

/// Verify derived PDA matches the expected account.
/// Isolated function to reduce stack usage.
#[inline(never)]
#[cfg(feature = "v2")]
pub fn verify_pda_match(
    derived_pda: &Pubkey,
    expected: &Pubkey,
) -> Result<(), crate::error::LightSdkError> {
    if *derived_pda != *expected {
        msg!(
            "Derived PDA does not match: expected {:?}, got {:?}",
            expected,
            derived_pda
        );
        return Err(crate::error::LightSdkError::ConstraintViolation);
    }
    Ok(())
}

/// Zero-copy variant: derive seeds, verify PDA, create account, and write directly to zero-copy buffer.
/// This avoids returning intermediate structs that cause stack overflow.
/// Takes compressed_meta_no_address by reference and computes compressed_meta internally to reduce caller stack usage.
#[inline(never)]
#[cfg(all(feature = "v2", feature = "cpi-context"))]
pub fn derive_verify_create_and_write_pda<'a, 'info, 'c, T, CtxSeeds, SeedParams>(
    program_id: &Pubkey,
    data: &T,
    ctx_seeds: &CtxSeeds,
    seed_params: Option<&SeedParams>,
    default_params: &SeedParams,
    compressed_meta_no_address: &CompressedAccountMetaNoLamportsNoAddress,
    address_space: &Pubkey,
    solana_account: &AccountInfo<'info>,
    rent_sponsor: &AccountInfo<'info>,
    cpi_accounts: &CpiAccounts<'a, 'info>,
    zc_info: &mut light_compressed_account::instruction_data::with_account_info::ZCompressedAccountInfoMut<'c>,
) -> Result<bool, solana_program_error::ProgramError>
where
    T: Clone
        + crate::account::Size
        + LightDiscriminator
        + Default
        + AnchorSerialize
        + AnchorDeserialize
        + HasCompressionInfo
        + crate::interface::PdaSeedDerivation<CtxSeeds, SeedParams>
        + 'info,
{
    // Derive PDA seeds (keeps seeds_vec on this stack frame)
    let (seeds_vec, derived_pda) = if let Some(params) = seed_params {
        T::derive_pda_seeds_with_accounts(data, program_id, ctx_seeds, params)?
    } else {
        T::derive_pda_seeds_with_accounts(data, program_id, ctx_seeds, default_params)?
    };

    // Verify PDA matches
    verify_pda_match(&derived_pda, solana_account.key)?;

    // Compute compressed_meta with address inside this frame (not in caller)
    let compressed_meta = into_compressed_meta_with_address(
        compressed_meta_no_address,
        solana_account,
        *address_space,
        program_id,
    );

    // Create PDA (seeds_vec is converted to refs here and dropped after)
    // Clone data since prepare_account_for_decompression_with_vec_seeds takes ownership
    let result = prepare_account_for_decompression_with_vec_seeds(
        program_id,
        data.clone(),
        compressed_meta,
        solana_account,
        rent_sponsor,
        cpi_accounts,
        &seeds_vec,
    )?;

    // Write directly to zero-copy buffer if created
    if let Some(account_info) = result {
        write_to_zero_copy_buffer(account_info, zc_info)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Write CompressedAccountInfo directly to zero-copy buffer.
/// Isolated function to reduce stack usage.
#[inline(never)]
#[cfg(all(feature = "v2", feature = "cpi-context"))]
fn write_to_zero_copy_buffer(
    account_info: CompressedAccountInfo,
    zc_info: &mut light_compressed_account::instruction_data::with_account_info::ZCompressedAccountInfoMut<'_>,
) -> Result<(), solana_program_error::ProgramError> {
    // Extract address
    let address = account_info
        .address
        .ok_or(solana_program_error::ProgramError::InvalidAccountData)?;

    // Write address to zero-copy buffer
    if let Some(ref mut zc_addr) = zc_info.address {
        zc_addr.copy_from_slice(&address);
    }

    // Extract and write input
    if let Some(input) = account_info.input {
        if let Some(ref mut zc_input) = zc_info.input {
            zc_input.discriminator = input.discriminator;
            zc_input.data_hash = input.data_hash;
            zc_input.merkle_context.merkle_tree_pubkey_index =
                input.merkle_context.merkle_tree_pubkey_index;
            zc_input.merkle_context.queue_pubkey_index = input.merkle_context.queue_pubkey_index;
            zc_input
                .merkle_context
                .leaf_index
                .set(input.merkle_context.leaf_index);
            zc_input.merkle_context.prove_by_index = input.merkle_context.prove_by_index as u8;
            zc_input.root_index.set(input.root_index);
            zc_input.lamports.set(input.lamports);
        }
    }

    // Extract and write output
    if let Some(output) = account_info.output {
        if let Some(ref mut zc_output) = zc_info.output {
            zc_output.discriminator = output.discriminator;
            zc_output.data_hash = output.data_hash;
            zc_output.output_merkle_tree_index = output.output_merkle_tree_index;
            zc_output.lamports.set(output.lamports);
            // Output data is the PDA pubkey (same as address for decompressed PDAs)
            zc_output.data.copy_from_slice(&address);
        }
    }

    Ok(())
}
