use light_compressed_account::instruction_data::with_account_info::{
    CompressedAccountInfo, InAccountInfo,
};
use light_compressible::{rent::AccountRentState, DECOMPRESSED_PDA_DISCRIMINATOR};
use light_hasher::{sha256::Sha256BE, DataHasher, Hasher};
use light_sdk_types::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;
use solana_account_info::AccountInfo;
use solana_clock::Clock;
use solana_msg::msg;
use solana_pubkey::Pubkey;
use solana_sysvar::{rent::Rent, Sysvar};

use crate::{
    account::sha::LightAccount,
    compressible::compression_info::{CompressAs, HasCompressionInfo},
    cpi::v2::CpiAccounts,
    error::LightSdkError,
    instruction::account_meta::CompressedAccountMeta,
    AnchorDeserialize, AnchorSerialize, LightDiscriminator, ProgramError,
};

/// Set input for decompressed PDA format.
/// Isolated in separate function to reduce stack usage.
#[inline(never)]
#[cfg(feature = "v2")]
fn set_decompressed_pda_input(
    input: &mut InAccountInfo,
    pda_pubkey_bytes: &[u8; 32],
) -> Result<(), ProgramError> {
    input.data_hash = Sha256BE::hash(pda_pubkey_bytes)
        .map_err(LightSdkError::from)
        .map_err(ProgramError::from)?;
    input.discriminator = DECOMPRESSED_PDA_DISCRIMINATOR;
    Ok(())
}
/// Prepare account for compression.
///
/// # Arguments
/// * `program_id` - The program that owns the account
/// * `account_info` - The account to compress
/// * `account_data` - Mutable reference to the deserialized account data
/// * `compressed_account_meta` - Metadata for the compressed account
/// * `cpi_accounts` - Accounts for CPI to light system program
/// * `address_space` - Address space for validation
#[cfg(feature = "v2")]
pub fn prepare_account_for_compression<'info, A>(
    program_id: &Pubkey,
    account_info: &AccountInfo<'info>,
    account_data: &mut A,
    compressed_account_meta: &CompressedAccountMetaNoLamportsNoAddress,
    cpi_accounts: &CpiAccounts<'_, 'info>,
    address_space: &[Pubkey],
) -> std::result::Result<CompressedAccountInfo, ProgramError>
where
    A: DataHasher
        + LightDiscriminator
        + AnchorSerialize
        + AnchorDeserialize
        + Default
        + Clone
        + HasCompressionInfo
        + CompressAs,
    A::Output: DataHasher
        + LightDiscriminator
        + AnchorSerialize
        + AnchorDeserialize
        + HasCompressionInfo
        + Default
        + crate::interface::compression_info::CompressedInitSpace,
{
    use light_compressed_account::address::derive_address;

    // v2 address derive using PDA as seed
    let derived_c_pda = derive_address(
        &account_info.key.to_bytes(),
        &address_space[0].to_bytes(),
        &program_id.to_bytes(),
    );

    let meta_with_address = CompressedAccountMeta {
        tree_info: compressed_account_meta.tree_info,
        address: derived_c_pda,
        output_state_tree_index: compressed_account_meta.output_state_tree_index,
    };

    let current_slot = Clock::get()?.slot;
    // Rent-function gating: account must be compressible w.r.t. rent function (current+next epoch)
    let bytes = account_info.data_len() as u64;
    let current_lamports = account_info.lamports();
    let rent_exemption_lamports = Rent::get()
        .map_err(|_| LightSdkError::ConstraintViolation)?
        .minimum_balance(bytes as usize);
    let ci = account_data.compression_info()?;
    let last_claimed_slot = ci.last_claimed_slot();
    let rent_cfg = ci.rent_config;
    let state = AccountRentState {
        num_bytes: bytes,
        current_slot,
        current_lamports,
        last_claimed_slot,
    };
    if state
        .is_compressible(&rent_cfg, rent_exemption_lamports)
        .is_none()
    {
        msg!(
            "prepare_account_for_compression failed: \
            Account is not compressible by rent function. \
            slot: {}, lamports: {}, bytes: {}, rent_exemption_lamports: {}, last_claimed_slot: {}, rent_config: {:?}",
            current_slot,
            current_lamports,
            bytes,
            rent_exemption_lamports,
            last_claimed_slot,
            rent_cfg
        );
        return Err(LightSdkError::ConstraintViolation.into());
    }

    account_data.compression_info_mut()?.set_compressed();
    {
        let mut data = account_info
            .try_borrow_mut_data()
            .map_err(|_| LightSdkError::ConstraintViolation)?;
        let writer = &mut &mut data[..];
        account_data.serialize(writer).map_err(|e| {
            msg!("Failed to serialize account data: {}", e);
            LightSdkError::ConstraintViolation
        })?;
    }

    let owner_program_id = cpi_accounts.self_program_id();
    let mut compressed_account =
        LightAccount::<A::Output>::new_empty(&owner_program_id, &meta_with_address)?;

    let compressed_data = match account_data.compress_as() {
        std::borrow::Cow::Borrowed(data) => data.clone(),
        std::borrow::Cow::Owned(data) => data,
    };
    compressed_account.account = compressed_data;
    // Set compression_info to compressed state before hashing
    // This ensures the hash includes the compressed state marker
    *compressed_account.account.compression_info_mut_opt() =
        Some(crate::compressible::compression_info::CompressionInfo::compressed());
    {
        use crate::interface::compression_info::CompressedInitSpace;
        let __lp_size = 8 + <A::Output as CompressedInitSpace>::COMPRESSED_INIT_SPACE;
        if __lp_size > 800 {
            msg!(
                "Compressed account would exceed 800-byte limit ({} bytes)",
                __lp_size
            );
            return Err(LightSdkError::ConstraintViolation.into());
        }
    }

    let mut account_info_result = compressed_account.to_account_info()?;

    // Fix input to use the decompressed PDA format:
    // - discriminator: DECOMPRESSED_PDA_DISCRIMINATOR
    // - data_hash: Sha256BE(pda_pubkey)
    if let Some(input) = account_info_result.input.as_mut() {
        set_decompressed_pda_input(input, &account_info.key.to_bytes())?;
    }

    Ok(account_info_result)
}

/// Prepare Pod (zero-copy) account for compression.
///
/// This function is the Pod equivalent of `prepare_account_for_compression`,
/// designed for accounts that use `bytemuck::Pod` instead of Borsh serialization.
///
/// # Key Differences from Borsh Version
///
/// - Uses `bytemuck::bytes_of()` instead of Borsh serialization
/// - Uses `core::mem::size_of::<A>()` for static size calculation
/// - Writes Pod bytes directly instead of serializing
/// - More efficient for accounts with fixed-size layout
///
/// # Type Requirements
///
/// - `A` must implement `bytemuck::Pod` and `bytemuck::Zeroable`
/// - `A` must be `#[repr(C)]` for predictable field layout
/// - `A` must implement `PodCompressionInfoField` for compression state management
///
/// # Arguments
/// * `program_id` - The program that owns the account
/// * `account_info` - The account to compress
/// * `account_data` - Mutable reference to the Pod account data
/// * `compressed_account_meta` - Metadata for the compressed account
/// * `cpi_accounts` - Accounts for CPI to light system program
/// * `address_space` - Address space for validation
#[cfg(feature = "v2")]
pub fn prepare_account_for_compression_pod<'info, A>(
    program_id: &Pubkey,
    account_info: &AccountInfo<'info>,
    account_data: &mut A,
    compressed_account_meta: &CompressedAccountMetaNoLamportsNoAddress,
    _cpi_accounts: &CpiAccounts<'_, 'info>,
    address_space: &[Pubkey],
) -> std::result::Result<CompressedAccountInfo, ProgramError>
where
    A: bytemuck::Pod
        + bytemuck::Zeroable
        + Copy
        + LightDiscriminator
        + crate::interface::compression_info::PodCompressionInfoField
        + Default,
{
    use crate::instruction::account_meta::CompressedAccountMetaTrait;
    use crate::interface::compression_info::{CompressionInfo as SdkCompressionInfo, CompressionState};
    use light_compressed_account::{
        address::derive_address,
        compressed_account::PackedMerkleContext,
        instruction_data::with_account_info::{InAccountInfo, OutAccountInfo},
    };
    use light_hasher::{Hasher, Sha256};

    // Default data hash for empty accounts (same as in account.rs)
    const DEFAULT_DATA_HASH: [u8; 32] = [0u8; 32];

    // v2 address derive using PDA as seed
    let derived_c_pda = derive_address(
        &account_info.key.to_bytes(),
        &address_space[0].to_bytes(),
        &program_id.to_bytes(),
    );

    let meta_with_address = CompressedAccountMeta {
        tree_info: compressed_account_meta.tree_info,
        address: derived_c_pda,
        output_state_tree_index: compressed_account_meta.output_state_tree_index,
    };

    let current_slot = Clock::get()?.slot;
    // Rent-function gating: account must be compressible w.r.t. rent function (current+next epoch)
    let bytes = account_info.data_len() as u64;
    let current_lamports = account_info.lamports();
    let rent_exemption_lamports = Rent::get()
        .map_err(|_| LightSdkError::ConstraintViolation)?
        .minimum_balance(bytes as usize);

    // Access the SDK compression info field directly (24 bytes)
    let compression_info_offset = A::COMPRESSION_INFO_OFFSET;
    let account_bytes = bytemuck::bytes_of(account_data);
    let compression_info_bytes =
        &account_bytes[compression_info_offset..compression_info_offset + core::mem::size_of::<SdkCompressionInfo>()];
    let sdk_ci: &SdkCompressionInfo = bytemuck::from_bytes(compression_info_bytes);

    let last_claimed_slot = sdk_ci.last_claimed_slot;
    let rent_cfg = sdk_ci.rent_config;
    let state = AccountRentState {
        num_bytes: bytes,
        current_slot,
        current_lamports,
        last_claimed_slot,
    };
    if state
        .is_compressible(&rent_cfg, rent_exemption_lamports)
        .is_none()
    {
        msg!(
            "prepare_account_for_compression_pod failed: \
            Account is not compressible by rent function. \
            slot: {}, lamports: {}, bytes: {}, rent_exemption_lamports: {}, last_claimed_slot: {}, rent_config: {:?}",
            current_slot,
            current_lamports,
            bytes,
            rent_exemption_lamports,
            last_claimed_slot,
            rent_cfg
        );
        return Err(LightSdkError::ConstraintViolation.into());
    }

    // Set compression state to compressed in the account data
    // We need to modify the Pod struct in place
    {
        let mut data = account_info
            .try_borrow_mut_data()
            .map_err(|_| LightSdkError::ConstraintViolation)?;

        // Skip discriminator (8 bytes) to get to the Pod data
        let discriminator_len = A::LIGHT_DISCRIMINATOR.len();
        let pod_data = &mut data[discriminator_len..];

        // Mark as compressed using SDK CompressionInfo (24 bytes)
        let compressed_info = SdkCompressionInfo {
            last_claimed_slot: sdk_ci.last_claimed_slot,
            lamports_per_write: sdk_ci.lamports_per_write,
            config_version: sdk_ci.config_version,
            state: CompressionState::Compressed, // Mark as compressed
            _padding: 0,
            rent_config: sdk_ci.rent_config,
        };

        let info_bytes = bytemuck::bytes_of(&compressed_info);
        let offset = A::COMPRESSION_INFO_OFFSET;
        let end = offset + core::mem::size_of::<SdkCompressionInfo>();
        pod_data[offset..end].copy_from_slice(info_bytes);
    }

    // Update the local copy with CANONICAL compressed CompressionInfo for hashing
    // Use CompressionInfo::compressed() for hash consistency with decompression
    // (decompression uses unpack_stripped which inserts the same canonical bytes)
    let mut compressed_data = *account_data;
    {
        let compressed_bytes: &mut [u8] = bytemuck::bytes_of_mut(&mut compressed_data);
        let offset = A::COMPRESSION_INFO_OFFSET;
        let end = offset + core::mem::size_of::<SdkCompressionInfo>();

        // Use canonical compressed value (consistent with Borsh path)
        let compressed_info = SdkCompressionInfo::compressed();
        let info_bytes = bytemuck::bytes_of(&compressed_info);
        compressed_bytes[offset..end].copy_from_slice(info_bytes);
    }

    // Hash the FULL bytes for output hash calculation (consistent with Borsh path)
    // Discriminator is NOT included in hash per protocol convention
    let compressed_bytes = bytemuck::bytes_of(&compressed_data);
    let mut output_data_hash = Sha256::hash(compressed_bytes).map_err(LightSdkError::from)?;
    output_data_hash[0] = 0; // Zero first byte per protocol convention

    // Strip CompressionInfo bytes to save 24 bytes per account in instruction data
    // The hash is computed from full bytes, but we only transmit stripped bytes
    let stripped_bytes = A::pack_stripped(&compressed_data);

    // Size check
    let account_size = 8 + core::mem::size_of::<A>();
    if account_size > 800 {
        msg!(
            "Compressed account would exceed 800-byte limit ({} bytes)",
            account_size
        );
        return Err(LightSdkError::ConstraintViolation.into());
    }

    // Build input account info - represents the empty compressed account from init
    // This is required for the system program to find the address in context.addresses
    let tree_info = compressed_account_meta.tree_info;
    let input_account_info = InAccountInfo {
        data_hash: DEFAULT_DATA_HASH,
        lamports: 0,
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
            queue_pubkey_index: tree_info.queue_pubkey_index,
            leaf_index: tree_info.leaf_index,
            prove_by_index: tree_info.prove_by_index,
        },
        root_index: compressed_account_meta.get_root_index().unwrap_or_default(),
        discriminator: [0u8; 8], // Empty account marker
    };

    // Build output account info for compression
    // Use stripped_bytes which saves 24 bytes (CompressionInfo size) per account
    let output_account_info = OutAccountInfo {
        lamports: 0,
        output_merkle_tree_index: meta_with_address.output_state_tree_index,
        discriminator: A::LIGHT_DISCRIMINATOR,
        data: stripped_bytes,
        data_hash: output_data_hash,
    };

    Ok(CompressedAccountInfo {
        address: Some(meta_with_address.address),
        input: Some(input_account_info),
        output: Some(output_account_info),
    })
}
