#![allow(clippy::all)] // TODO: Remove.

use light_compressed_account::{
    address::derive_address,
    compressed_account::PackedMerkleContext,
    instruction_data::with_account_info::{CompressedAccountInfo, InAccountInfo, OutAccountInfo},
};
use light_hasher::{Hasher, Sha256};
use light_program_profiler::profile;
use light_sdk_types::instruction::account_meta::{
    CompressedAccountMeta, CompressedAccountMetaNoLamportsNoAddress,
};
use solana_account_info::AccountInfo;
use solana_cpi::invoke_signed;
use solana_msg::msg;
use solana_pubkey::Pubkey;
use solana_system_interface::instruction as system_instruction;
use solana_sysvar::rent::Rent;

use crate::{
    account::LightAccountInner,
    compressible::compression_info::{
        CompressionInfo, CompressionInfoField, HasCompressionInfo, PodCompressionInfoField,
    },
    cpi::v2::CpiAccounts,
    error::LightSdkError,
    AnchorDeserialize, AnchorSerialize, LightDiscriminator,
};

/// Compute the data hash for compressed account verification.
///
/// This is the canonical way to hash account data for Light Protocol:
/// 1. Hash the raw data bytes (WITHOUT discriminator prefix)
/// 2. Zero the first byte per protocol convention
///
/// Both Borsh and Pod decompression paths must use this same logic
/// to ensure hash consistency.
///
/// # Arguments
/// * `data_bytes` - Raw account data bytes (discriminator NOT included)
///
/// # Returns
/// * 32-byte hash with first byte zeroed
#[inline]
pub fn compute_data_hash(data_bytes: &[u8]) -> Result<[u8; 32], LightSdkError> {
    let mut hash = Sha256::hash(data_bytes).map_err(LightSdkError::from)?;
    hash[0] = 0; // Zero first byte per protocol convention
    Ok(hash)
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

/// Cold path: Account already has lamports (e.g., attacker donation).
/// Uses Assign + Allocate + Transfer instead of CreateAccount which would fail.
#[cold]
fn create_pda_account_with_lamports<'info>(
    rent_sponsor: &AccountInfo<'info>,
    solana_account: &AccountInfo<'info>,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
    seeds: &[&[u8]],
    system_program: &AccountInfo<'info>,
) -> Result<(), LightSdkError> {
    let current_lamports = solana_account.lamports();

    // Assign owner
    let assign_ix = system_instruction::assign(solana_account.key, owner);
    invoke_signed(
        &assign_ix,
        &[solana_account.clone(), system_program.clone()],
        &[seeds],
    )
    .map_err(LightSdkError::ProgramError)?;

    // Allocate space
    let allocate_ix = system_instruction::allocate(solana_account.key, space);
    invoke_signed(
        &allocate_ix,
        &[solana_account.clone(), system_program.clone()],
        &[seeds],
    )
    .map_err(LightSdkError::ProgramError)?;

    // Transfer remaining lamports for rent-exemption if needed
    if lamports > current_lamports {
        let transfer_ix = system_instruction::transfer(
            rent_sponsor.key,
            solana_account.key,
            lamports - current_lamports,
        );
        invoke_signed(
            &transfer_ix,
            &[
                rent_sponsor.clone(),
                solana_account.clone(),
                system_program.clone(),
            ],
            &[],
        )
        .map_err(LightSdkError::ProgramError)?;
    }

    Ok(())
}

/// Creates a PDA account, handling the case where the account already has lamports.
///
/// This function handles the edge case where an attacker might have donated lamports
/// to the PDA address before decompression. In that case, `CreateAccount` would fail,
/// so we fall back to `Assign + Allocate + Transfer`.
#[inline(never)]
pub fn create_pda_account<'info>(
    rent_sponsor: &AccountInfo<'info>,
    solana_account: &AccountInfo<'info>,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
    seeds: &[&[u8]],
    system_program: &AccountInfo<'info>,
) -> Result<(), LightSdkError> {
    // Cold path: account already has lamports (e.g., attacker donation)
    if solana_account.lamports() > 0 {
        return create_pda_account_with_lamports(
            rent_sponsor,
            solana_account,
            lamports,
            space,
            owner,
            seeds,
            system_program,
        );
    }

    // Normal path: CreateAccount
    let create_account_ix = system_instruction::create_account(
        rent_sponsor.key,
        solana_account.key,
        lamports,
        space,
        owner,
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
    .map_err(LightSdkError::ProgramError)
}

/// Helper function to decompress a compressed account into a PDA
/// idempotently with seeds.
#[inline(never)]
#[profile]
pub fn prepare_account_for_decompression_idempotent<'a, 'info, T>(
    program_id: &Pubkey,
    mut account: T,
    compressed_meta: CompressedAccountMeta,
    solana_account: &AccountInfo<'info>,
    rent_sponsor: &AccountInfo<'info>,
    cpi_accounts: &CpiAccounts<'a, 'info>,
    signer_seeds: &[&[u8]],
    rent: &Rent,
    current_slot: u64,
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
        + CompressionInfoField
        + 'info,
{
    // Check if account is already initialized by examining discriminator
    if !solana_account.data_is_empty() {
        let data = solana_account.try_borrow_data()?;
        // If discriminator is NOT zeroed, account is already initialized - skip
        if light_account_checks::checks::check_data_is_zeroed::<8>(&data).is_err() {
            msg!("Account already initialized, skipping");
            return Ok(None);
        }
        // Discriminator is zeroed but data exists - unexpected state, let create_pda fail
    }
    *account.compression_info_mut_opt() = Some(CompressionInfo::compressed());
    let (light_account, data) =
        LightAccountInner::<Sha256, T, true>::new_mut_inner(program_id, &compressed_meta, account)?;

    // Account space needs to include discriminator + serialized data
    // T::size() already includes the full Option<CompressionInfo> footprint
    let discriminator_len = T::LIGHT_DISCRIMINATOR.len();
    let space = discriminator_len + data.len();
    let rent_minimum_balance = rent.minimum_balance(space);

    create_pda_account(
        rent_sponsor,
        solana_account,
        rent_minimum_balance,
        space as u64,
        &cpi_accounts.self_program_id(),
        signer_seeds,
        cpi_accounts.system_program()?,
    )?;

    // Write discriminator + already-serialized data, then patch compression_info in place
    let mut account_data = solana_account.try_borrow_mut_data()?;
    let discriminator_len = T::LIGHT_DISCRIMINATOR.len();
    account_data[..discriminator_len].copy_from_slice(&T::LIGHT_DISCRIMINATOR);
    account_data[discriminator_len..space].copy_from_slice(&data);

    // Patch compression_info to decompressed state at the correct offset
    T::write_decompressed_info_to_slice(&mut account_data[discriminator_len..], current_slot)
        .map_err(|err| {
            msg!("Failed to write decompressed compression_info: {:?}", err);
            LightSdkError::Borsh
        })?;

    Ok(Some(light_account.to_account_info()?))
}

/// Helper function to decompress a compressed account into a PDA
/// idempotently with seeds. Optimized for Pod (zero-copy) accounts.
///
/// # Key Differences from Borsh Version
///
/// - Uses `std::mem::size_of::<T>()` for static size calculation
/// - Uses `bytemuck::bytes_of()` instead of Borsh serialization
/// - Patches CompressionInfo at fixed byte offset (no Option discriminant)
/// - More efficient for accounts with fixed-size layout
///
/// # Type Requirements
///
/// - `T` must implement `bytemuck::Pod` and `bytemuck::Zeroable`
/// - `T` must be `#[repr(C)]` for predictable field layout
/// - `T` must implement `PodCompressionInfoField` for compression state management
///
/// # Hash Consistency
///
/// Pod accounts use their own hashing path independent of Borsh accounts.
/// The hash is computed from `bytemuck::bytes_of(&account)`, which gives
/// the raw memory representation. This is consistent as long as:
/// - The same Pod type is used for compression and decompression
/// - No mixing between Pod and Borsh code paths for the same account type
#[inline(never)]
#[profile]
pub fn prepare_account_for_decompression_idempotent_pod<'a, 'info, T>(
    _program_id: &Pubkey,
    account: T,
    compressed_meta: CompressedAccountMeta,
    solana_account: &AccountInfo<'info>,
    rent_sponsor: &AccountInfo<'info>,
    cpi_accounts: &CpiAccounts<'a, 'info>,
    signer_seeds: &[&[u8]],
    rent: &Rent,
    current_slot: u64,
) -> Result<Option<CompressedAccountInfo>, LightSdkError>
where
    T: bytemuck::Pod
        + bytemuck::Zeroable
        + Copy
        + LightDiscriminator
        + PodCompressionInfoField
        + Default
        + 'info,
{
    // Check if account is already initialized by examining discriminator
    if !solana_account.data_is_empty() {
        let data = solana_account.try_borrow_data()?;
        // If discriminator is NOT zeroed, account is already initialized - skip
        if light_account_checks::checks::check_data_is_zeroed::<8>(&data).is_err() {
            msg!("Account already initialized, skipping");
            return Ok(None);
        }
        // Discriminator is zeroed but data exists - unexpected state, let create_pda fail
    }

    // Hash the FULL bytes for input verification (matches what's in Merkle tree)
    // During compression, we hashed full bytes with canonical CompressionInfo::compressed().
    // The account parameter was reconstructed via unpack_stripped, which inserted the
    // same canonical compressed bytes, so hashing full bytes will match.
    let full_bytes = bytemuck::bytes_of(&account);
    let input_data_hash = compute_data_hash(full_bytes)?;

    // Build input account info
    let tree_info = compressed_meta.tree_info;
    let input_account_info = InAccountInfo {
        data_hash: input_data_hash,
        lamports: 0,
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
            queue_pubkey_index: tree_info.queue_pubkey_index,
            leaf_index: tree_info.leaf_index,
            prove_by_index: tree_info.prove_by_index,
        },
        root_index: tree_info.root_index,
        discriminator: T::LIGHT_DISCRIMINATOR,
    };

    // Static size calculation - more efficient than dynamic
    let discriminator_len = T::LIGHT_DISCRIMINATOR.len();
    let space = discriminator_len + core::mem::size_of::<T>();
    let rent_minimum_balance = rent.minimum_balance(space);

    create_pda_account(
        rent_sponsor,
        solana_account,
        rent_minimum_balance,
        space as u64,
        &cpi_accounts.self_program_id(),
        signer_seeds,
        cpi_accounts.system_program()?,
    )?;

    // Write discriminator + raw Pod bytes (full bytes, not stripped)
    // The account was reconstructed from stripped bytes with zeros at CompressionInfo offset
    let full_bytes = bytemuck::bytes_of(&account);
    let mut account_data = solana_account.try_borrow_mut_data()?;
    account_data[..discriminator_len].copy_from_slice(&T::LIGHT_DISCRIMINATOR);
    account_data[discriminator_len..space].copy_from_slice(full_bytes);

    // Patch compression_info to decompressed state at fixed offset
    T::write_decompressed_info_to_slice_pod(&mut account_data[discriminator_len..], current_slot)
        .map_err(|err| {
            msg!("Failed to write decompressed compression_info: {:?}", err);
            LightSdkError::Borsh
        })?;

    // Build output account info
    let output_account_info = OutAccountInfo {
        lamports: 0,
        output_merkle_tree_index: compressed_meta.output_state_tree_index,
        discriminator: T::LIGHT_DISCRIMINATOR,
        data: Vec::new(),
        data_hash: [0u8; 32],
    };

    Ok(Some(CompressedAccountInfo {
        address: Some(compressed_meta.address),
        input: Some(input_account_info),
        output: Some(output_account_info),
    }))
}
