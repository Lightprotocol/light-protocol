use light_compressed_account::instruction_data::{
    data::NewAddressParamsAssignedPacked,
    with_account_info::{CompressedAccountInfo, OutAccountInfo},
};
use light_compressible::DECOMPRESSED_PDA_DISCRIMINATOR;
use light_hasher::{sha256::Sha256BE, DataHasher, Hasher};
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_pubkey::Pubkey;

use crate::{
    account::sha::LightAccount, compressible::compression_info::HasCompressionInfo,
    cpi::v2::CpiAccounts, error::LightSdkError, light_account_checks::AccountInfoTrait,
    AnchorDeserialize, AnchorSerialize, LightDiscriminator, ProgramError,
};

/// Set output for decompressed PDA format.
/// Isolated in separate function to reduce stack usage.
#[inline(never)]
#[cfg(feature = "v2")]
fn set_decompressed_pda_output(
    output: &mut OutAccountInfo,
    pda_pubkey_bytes: &[u8; 32],
) -> Result<(), ProgramError> {
    output.data = pda_pubkey_bytes.to_vec();
    output.data_hash = Sha256BE::hash(pda_pubkey_bytes)
        .map_err(LightSdkError::from)
        .map_err(ProgramError::from)?;
    output.discriminator = DECOMPRESSED_PDA_DISCRIMINATOR;
    Ok(())
}

/// Prepare a compressed account on init.
///
/// Does NOT close the PDA, does NOT invoke CPI.
///
/// # Arguments
/// * `account_info` - The PDA AccountInfo
/// * `account_data` - Mutable reference to deserialized account data
/// * `address` - The address for the compressed account
/// * `new_address_param` - Address parameters for the compressed account
/// * `output_state_tree_index` - Output state tree index
/// * `cpi_accounts` - Accounts for validation
/// * `address_space` - Address space for validation (can contain multiple tree
///   pubkeys)
/// * `with_data` - If true, copies account data to compressed account, if
///   false, creates empty compressed account
///
/// # Returns
/// CompressedAccountInfo
#[allow(clippy::too_many_arguments)]
#[cfg(feature = "v2")]
pub fn prepare_compressed_account_on_init<'info, A>(
    account_info: &AccountInfo<'info>,
    account_data: &mut A,
    compression_config: &crate::interface::LightConfig,
    address: [u8; 32],
    new_address_param: NewAddressParamsAssignedPacked,
    output_state_tree_index: u8,
    cpi_accounts: &CpiAccounts<'_, 'info>,
    address_space: &[Pubkey],
    with_data: bool,
) -> std::result::Result<CompressedAccountInfo, ProgramError>
where
    A: DataHasher
        + LightDiscriminator
        + AnchorSerialize
        + AnchorDeserialize
        + Default
        + Clone
        + HasCompressionInfo,
{
    // TODO: consider not supporting yet.
    // Fail-fast: with_data=true is not yet supported in macro-generated code
    // if with_data {
    //     msg!("with_data=true is not supported yet");
    //     return Err(LightSdkError::ConstraintViolation.into());
    // }

    let tree = cpi_accounts
        .get_tree_account_info(new_address_param.address_merkle_tree_account_index as usize)
        .map_err(|_| {
            msg!(
                "Failed to get tree account at index {}",
                new_address_param.address_merkle_tree_account_index
            );
            LightSdkError::ConstraintViolation
        })?
        .pubkey();
    if !address_space.iter().any(|a| a == &tree) {
        msg!("Address tree {} not in allowed address space", tree);
        return Err(LightSdkError::ConstraintViolation.into());
    }
    // Initialize CompressionInfo from config
    // Note: Rent sponsor is not stored per-account; compression always sends rent to config's rent_sponsor
    use solana_sysvar::{clock::Clock, Sysvar};
    let current_slot = Clock::get()?.slot;
    *account_data.compression_info_mut_opt() = Some(
        super::compression_info::CompressionInfo::new_from_config(compression_config, current_slot),
    );

    if with_data {
        account_data.compression_info_mut()?.set_compressed();
    } else {
        account_data
            .compression_info_mut()?
            .bump_last_claimed_slot()?;
    }
    {
        let mut data = account_info
            .try_borrow_mut_data()
            .map_err(|_| LightSdkError::ConstraintViolation)?;
        // Skip the 8-byte Anchor discriminator when serializing
        account_data.serialize(&mut &mut data[8..]).map_err(|e| {
            msg!("Failed to serialize account data: {}", e);
            LightSdkError::ConstraintViolation
        })?;
    }

    let owner_program_id = cpi_accounts.self_program_id();

    let mut compressed_account =
        LightAccount::<A>::new_init(&owner_program_id, Some(address), output_state_tree_index);

    if with_data {
        let mut compressed_data = account_data.clone();
        compressed_data.set_compression_info_none()?;
        compressed_account.account = compressed_data;
    } else {
        compressed_account.remove_data();
    }

    let mut account_info_result = compressed_account.to_account_info()?;

    // For decompressed PDAs (with_data = false), store the PDA pubkey in data
    // and set the decompressed discriminator
    if !with_data {
        if let Some(output) = account_info_result.output.as_mut() {
            set_decompressed_pda_output(output, &account_info.key())?;
        }
    }

    Ok(account_info_result)
}

/// Prepare a compressed Pod (zero-copy) account on init.
///
/// This function is the Pod equivalent of `prepare_compressed_account_on_init`,
/// designed for accounts that use `bytemuck::Pod` instead of Borsh serialization.
///
/// Does NOT close the PDA, does NOT invoke CPI.
///
/// # Key Differences from Borsh Version
///
/// - Uses `bytemuck::bytes_of()` instead of Borsh serialization
/// - Uses `core::mem::size_of::<A>()` for static size calculation
/// - Writes Pod bytes directly instead of serializing
/// - Uses non-optional `CompressionInfo` where `config_account_version=0` means uninitialized
///
/// # Type Requirements
///
/// - `A` must implement `bytemuck::Pod` and `bytemuck::Zeroable`
/// - `A` must be `#[repr(C)]` for predictable field layout
/// - `A` must implement `PodCompressionInfoField` for compression state management
///
/// # Arguments
/// * `account_info` - The PDA AccountInfo
/// * `account_data` - Mutable reference to Pod account data
/// * `compression_config` - Configuration for compression parameters
/// * `address` - The address for the compressed account
/// * `new_address_param` - Address parameters for the compressed account
/// * `output_state_tree_index` - Output state tree index
/// * `cpi_accounts` - Accounts for validation
/// * `address_space` - Address space for validation (can contain multiple tree pubkeys)
/// * `with_data` - If true, copies account data to compressed account, if false, creates empty
#[allow(clippy::too_many_arguments)]
#[cfg(feature = "v2")]
pub fn prepare_compressed_account_on_init_pod<'info, A>(
    account_info: &AccountInfo<'info>,
    account_data: &mut A,
    compression_config: &crate::interface::LightConfig,
    address: [u8; 32],
    new_address_param: NewAddressParamsAssignedPacked,
    output_state_tree_index: u8,
    cpi_accounts: &CpiAccounts<'_, 'info>,
    address_space: &[Pubkey],
    with_data: bool,
) -> std::result::Result<CompressedAccountInfo, ProgramError>
where
    A: bytemuck::Pod
        + bytemuck::Zeroable
        + Copy
        + LightDiscriminator
        + crate::interface::compression_info::PodCompressionInfoField
        + Default,
{
    use crate::interface::compression_info::{CompressionInfo as SdkCompressionInfo, CompressionState};
    use light_compressed_account::instruction_data::with_account_info::OutAccountInfo;
    use light_hasher::{Hasher, Sha256};
    use solana_sysvar::{clock::Clock, Sysvar};

    // Validate address tree is in allowed address space
    let tree = cpi_accounts
        .get_tree_account_info(new_address_param.address_merkle_tree_account_index as usize)
        .map_err(|_| {
            msg!(
                "Failed to get tree account at index {}",
                new_address_param.address_merkle_tree_account_index
            );
            LightSdkError::ConstraintViolation
        })?
        .pubkey();
    if !address_space.iter().any(|a| a == &tree) {
        msg!("Address tree {} not in allowed address space", tree);
        return Err(LightSdkError::ConstraintViolation.into());
    }

    let current_slot = Clock::get()?.slot;

    // Create SDK CompressionInfo from config (24 bytes)
    // state = Decompressed means initialized/decompressed
    // state = Compressed means compressed
    let base_compression_info = SdkCompressionInfo {
        last_claimed_slot: current_slot,
        lamports_per_write: compression_config.write_top_up, // Already u32 in LightConfig
        config_version: (compression_config.version as u16).max(1), // Ensure at least 1 for initialized
        state: CompressionState::Decompressed,
        _padding: 0,
        rent_config: compression_config.rent_config,
    };

    // If with_data, mark as compressed
    let final_compression_info = if with_data {
        SdkCompressionInfo {
            state: CompressionState::Compressed, // Compressed state
            ..base_compression_info
        }
    } else {
        base_compression_info
    };

    // Write compression info to account data in memory.
    // For AccountLoader (zero-copy), account_data is a mutable reference to the
    // account buffer (after discriminator), so this writes directly to the account.
    {
        let account_bytes: &mut [u8] = bytemuck::bytes_of_mut(account_data);
        let offset = A::COMPRESSION_INFO_OFFSET;
        let end = offset + core::mem::size_of::<SdkCompressionInfo>();
        let info_bytes = bytemuck::bytes_of(&final_compression_info);
        account_bytes[offset..end].copy_from_slice(info_bytes);
    }

    let _owner_program_id = cpi_accounts.self_program_id();
    let _ = account_info; // Keep for API consistency with non-pod version

    if with_data {
        // Create a copy with CANONICAL compressed CompressionInfo for hashing
        // Use CompressionInfo::compressed() for hash consistency with decompression
        // (decompression uses unpack_stripped which inserts the same canonical bytes)
        let mut hash_data = *account_data;
        {
            let hash_bytes: &mut [u8] = bytemuck::bytes_of_mut(&mut hash_data);
            let offset = A::COMPRESSION_INFO_OFFSET;
            let end = offset + core::mem::size_of::<SdkCompressionInfo>();
            let canonical_compressed = SdkCompressionInfo::compressed();
            let info_bytes = bytemuck::bytes_of(&canonical_compressed);
            hash_bytes[offset..end].copy_from_slice(info_bytes);
        }

        // Hash the FULL bytes for output hash calculation (consistent with Borsh path)
        // Discriminator is NOT included in hash per protocol convention
        let full_bytes = bytemuck::bytes_of(&hash_data);
        let mut output_data_hash = Sha256::hash(full_bytes).map_err(LightSdkError::from)?;
        output_data_hash[0] = 0; // Zero first byte per protocol convention

        // Strip CompressionInfo bytes to save 24 bytes per account in instruction data
        // The hash is computed from full bytes, but we only transmit stripped bytes
        let stripped_bytes = A::pack_stripped(&hash_data);

        // Size check
        let account_size = 8 + core::mem::size_of::<A>();
        if account_size > 800 {
            msg!(
                "Compressed account would exceed 800-byte limit ({} bytes)",
                account_size
            );
            return Err(LightSdkError::ConstraintViolation.into());
        }

        // Use stripped_bytes which saves 24 bytes (CompressionInfo size) per account
        let output_account_info = OutAccountInfo {
            lamports: 0,
            output_merkle_tree_index: output_state_tree_index,
            discriminator: A::LIGHT_DISCRIMINATOR,
            data: stripped_bytes,
            data_hash: output_data_hash,
        };

        Ok(CompressedAccountInfo {
            address: Some(address),
            input: None,
            output: Some(output_account_info),
        })
    } else {
        // Create empty compressed account (no data, just address registration)
        // Use [0u8; 8] discriminator for empty accounts (consistent with Borsh version)
        Ok(CompressedAccountInfo {
            address: Some(address),
            input: None,
            output: Some(OutAccountInfo {
                lamports: 0,
                output_merkle_tree_index: output_state_tree_index,
                discriminator: [0u8; 8],
                data: vec![],
                data_hash: [0u8; 32],
            }),
        })
    }
}
