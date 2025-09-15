use anchor_lang::prelude::ProgramError;
use light_account_checks::AccountInfoTrait;
use light_compressible::config::CompressibleConfig;
use light_ctoken_types::{
    instructions::extensions::compressible::CompressibleExtensionInstructionData,
    state::{CompressibleExtension, ZCompressibleExtensionMut},
    COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
};
use light_profiler::profile;
use light_zero_copy::traits::ZeroCopyAtMut;
#[cfg(target_os = "solana")]
use pinocchio::sysvars::{clock::Clock, Sysvar};
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey};

use crate::ErrorCode;

/// Initialize a token account using spl-pod with zero balance and default settings
#[profile]
pub fn initialize_token_account(
    token_account_info: &AccountInfo,
    mint_pubkey: &[u8; 32],
    owner_pubkey: &[u8; 32],
    compressible_config: Option<CompressibleExtensionInstructionData>,
    compressible_config_account: Option<&CompressibleConfig>,
    // account is compressible but with custom fee payer -> rent recipient is fee payer
    custom_fee_payer: Option<Pubkey>,
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
    if actual_size < required_size {
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
        // Split to get the actual CompressibleExtension data starting at byte 7
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

        // Create zero-copy mutable reference to CompressibleExtension
        let (mut compressible_extension, _) =
            CompressibleExtension::zero_copy_at_mut(compressible_data).map_err(|e| {
                msg!(
                    "Failed to create CompressibleExtension zero-copy reference: {:?}",
                    e
                );
                ProgramError::InvalidAccountData
            })?;

        configure_compressible_extension(
            &mut compressible_extension,
            compressible_config,
            compressible_config_account,
            custom_fee_payer,
        )?;
    }

    Ok(())
}

#[profile]
#[inline(always)]
fn configure_compressible_extension(
    compressible_extension: &mut ZCompressibleExtensionMut<'_>,
    compressible_config: CompressibleExtensionInstructionData,
    compressible_config_account: &CompressibleConfig,
    custom_fee_payer: Option<Pubkey>,
) -> Result<(), ProgramError> {
    // Set version to 1 (initialized)
    compressible_extension.version = compressible_config_account.version.into();

    #[cfg(target_os = "solana")]
    let current_slot = Clock::get()
        .map_err(|_| ProgramError::UnsupportedSysvar)?
        .slot;
    #[cfg(not(target_os = "solana"))]
    let current_slot = 1;
    compressible_extension.last_claimed_slot = current_slot.into();
    // Initialize RentConfig with default values
    compressible_extension.rent_config.min_rent =
        compressible_config_account.rent_config.min_rent.into();
    compressible_extension
        .rent_config
        .full_compression_incentive = compressible_config_account
        .rent_config
        .full_compression_incentive
        .into();
    compressible_extension.rent_config.rent_per_byte =
        compressible_config_account.rent_config.rent_per_byte;
    // Set the rent_authority, rent_recipient and write_top_up_lamports
    compressible_extension.rent_authority = compressible_config_account.rent_authority.to_bytes();
    if let Some(custom_fee_payer) = custom_fee_payer {
        // If the fee payer is a custom fee payer it becomes the rent recipient.
        // In this case the rent mechanism stay the same,
        // the account can be compressed and closed by a forester,
        // rent rewards cannot be claimed by the forester.
        compressible_extension.rent_recipient = custom_fee_payer;
    } else {
        compressible_extension.rent_recipient =
            compressible_config_account.rent_recipient.to_bytes();
    }

    compressible_extension
        .write_top_up_lamports
        .set(compressible_config.write_top_up);
    compressible_extension.compress_to_pubkey =
        compressible_config.compress_to_account_pubkey.is_some() as u8;
    Ok(())
}
