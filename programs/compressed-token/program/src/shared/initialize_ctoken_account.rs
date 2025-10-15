use anchor_lang::prelude::ProgramError;
use light_account_checks::AccountInfoTrait;
use light_compressible::{compression_info::ZCompressionInfoMut, config::CompressibleConfig};
use light_ctoken_types::{
    instructions::extensions::compressible::CompressibleExtensionInstructionData,
    state::CompressionInfo, COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
};
use light_program_profiler::profile;
use light_zero_copy::traits::ZeroCopyAtMut;
#[cfg(target_os = "solana")]
use pinocchio::sysvars::{clock::Clock, Sysvar};
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey};

use crate::ErrorCode;

/// Initialize a token account using spl-pod with zero balance and default settings
#[profile]
pub fn initialize_ctoken_account(
    token_account_info: &AccountInfo,
    mint_pubkey: &[u8; 32],
    owner_pubkey: &[u8; 32],
    compressible_config: Option<CompressibleExtensionInstructionData>,
    compressible_config_account: Option<&CompressibleConfig>,
    // account is compressible but with custom fee payer -> rent recipient is fee payer
    custom_rent_payer: Option<Pubkey>,
) -> Result<(), ProgramError> {
    let required_size = if compressible_config.is_none() {
        165
    } else {
        COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize
    };
    // Access the token account data as mutable bytes
    let mut token_account_data = AccountInfoTrait::try_borrow_mut_data(token_account_info)?;
    let actual_size = token_account_data.len();

    // Check account size before attempting to initialize
    if actual_size != required_size {
        msg!(
            "Account too small: required {} bytes, got {} bytes",
            required_size,
            actual_size
        );
        return Err(ErrorCode::InsufficientAccountSize.into());
    }

    // Manually initialize the token account at the correct offsets
    // SPL Token Account Layout (165 bytes total):
    // mint: 32 bytes (offset 0-31)
    // owner: 32 bytes (offset 32-63)
    // state: 1 byte (offset 108)
    // Account is already zeroed, only need to set these 3 fields

    let (base_token_bytes, extension_bytes) = token_account_data.split_at_mut(165);

    if base_token_bytes[108] != 0 {
        msg!("Token account already initialized");
        return Err(ErrorCode::AlreadyInitialized.into());
    }

    // Copy mint (32 bytes at offset 0)
    base_token_bytes[0..32].copy_from_slice(mint_pubkey);

    // Copy owner (32 bytes at offset 32)
    base_token_bytes[32..64].copy_from_slice(owner_pubkey);

    // Set state to Initialized (1 byte at offset 108)
    base_token_bytes[108] = 1;

    // Configure compressible extension if present
    if let Some(compressible_config) = compressible_config {
        let compressible_config_account =
            compressible_config_account.ok_or(ErrorCode::InvalidCompressAuthority)?;
        // Split to get the actual CompressionInfo data starting at byte 7
        let (extension_bytes, compressible_data) = extension_bytes.split_at_mut(7);

        // Manually set extension metadata
        // Byte 0: AccountType::Account = 2
        extension_bytes[0] = 2;

        // Byte 1: Option::Some = 1 (for Option<Vec<ExtensionStruct>>)
        extension_bytes[1] = 1;

        // Bytes 2-5: Vec length = 1 (little-endian u32)
        extension_bytes[2..6].copy_from_slice(&[1, 0, 0, 0]);

        // Byte 6: Compressible enum discriminator = 26
        extension_bytes[6] = 26;

        // Create zero-copy mutable reference to CompressionInfo
        let (mut compressible_extension, _) = CompressionInfo::zero_copy_at_mut(compressible_data)
            .map_err(|e| {
                msg!(
                    "Failed to create CompressionInfo zero-copy reference: {:?}",
                    e
                );
                ProgramError::InvalidAccountData
            })?;

        configure_compressible_extension(
            &mut compressible_extension,
            compressible_config,
            compressible_config_account,
            custom_rent_payer,
        )?;
    }

    Ok(())
}

#[profile]
#[inline(always)]
fn configure_compressible_extension(
    compressible_extension: &mut ZCompressionInfoMut<'_>,
    compressible_config: CompressibleExtensionInstructionData,
    compressible_config_account: &CompressibleConfig,
    custom_rent_payer: Option<Pubkey>,
) -> Result<(), ProgramError> {
    // Set config_account_version
    compressible_extension.config_account_version = compressible_config_account.version.into();

    #[cfg(target_os = "solana")]
    let current_slot = Clock::get()
        .map_err(|_| ProgramError::UnsupportedSysvar)?
        .slot;
    #[cfg(not(target_os = "solana"))]
    let current_slot = 1;
    compressible_extension.last_claimed_slot = current_slot.into();
    // Initialize RentConfig with default values
    compressible_extension.rent_config.base_rent =
        compressible_config_account.rent_config.base_rent.into();
    compressible_extension.rent_config.compression_cost = compressible_config_account
        .rent_config
        .compression_cost
        .into();
    compressible_extension
        .rent_config
        .lamports_per_byte_per_epoch = compressible_config_account
        .rent_config
        .lamports_per_byte_per_epoch;
    compressible_extension.rent_config.max_funded_epochs =
        compressible_config_account.rent_config.max_funded_epochs;

    // Set the compression_authority, rent_sponsor and lamports_per_write
    compressible_extension.compression_authority =
        compressible_config_account.compression_authority.to_bytes();
    if let Some(custom_rent_payer) = custom_rent_payer {
        // The custom rent payer is the rent recipient.
        // In this case the rent mechanism stay the same,
        // the account can be compressed and closed by a forester,
        // rent rewards cannot be claimed by the forester.
        compressible_extension.rent_sponsor = custom_rent_payer;
    } else {
        compressible_extension.rent_sponsor = compressible_config_account.rent_sponsor.to_bytes();
    }

    compressible_extension
        .lamports_per_write
        .set(compressible_config.write_top_up);
    compressible_extension.compress_to_pubkey =
        compressible_config.compress_to_account_pubkey.is_some() as u8;
    // Validate token_account_version is ShaFlat (3)
    if compressible_config.token_account_version != 3 {
        msg!(
            "Invalid token_account_version: {}. Only version 3 (ShaFlat) is supported",
            compressible_config.token_account_version
        );
        return Err(ProgramError::InvalidInstructionData);
    }
    compressible_extension.account_version = compressible_config.token_account_version;
    Ok(())
}
