use anchor_lang::prelude::ProgramError;
use light_account_checks::AccountInfoTrait;
use light_compressible::config::CompressibleConfig;
use light_ctoken_types::{
    instructions::extensions::compressible::CompressibleExtensionInstructionData,
    state::{
        CompressedToken, CompressedTokenConfig, CompressibleExtensionConfig, ExtensionStructConfig,
        ZExtensionStructMut,
    },
};
use light_zero_copy::ZeroCopyNew;
#[cfg(target_os = "solana")]
use pinocchio::sysvars::{clock::Clock, Sysvar};
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey};

use crate::ErrorCode;

/// Initialize a token account using spl-pod with zero balance and default settings
pub fn initialize_token_account(
    token_account_info: &AccountInfo,
    mint_pubkey: &[u8; 32],
    owner_pubkey: &[u8; 32],
    compressible_config: Option<CompressibleExtensionInstructionData>,
    compressible_config_account: Option<CompressibleConfig>,
    // account is compressible but with custom fee payer -> rent recipient is fee payer
    custom_fee_payer: Option<Pubkey>,
) -> Result<(), ProgramError> {
    let current_lamports: u64 = *token_account_info
        .try_borrow_lamports()
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;
    msg!(
        "Initializing token account with {} lamports",
        current_lamports
    );
    // Access the token account data as mutable bytes
    let mut token_account_data = AccountInfoTrait::try_borrow_mut_data(token_account_info)?;

    // Create configuration for the compressed token
    let extensions = if compressible_config.is_some() {
        vec![ExtensionStructConfig::Compressible(
            CompressibleExtensionConfig { rent_config: () },
        )]
    } else {
        vec![]
    };

    let config = CompressedTokenConfig {
        // Start with zero balance
        delegate: false,        // No delegate
        is_native: false,       // Not a native token
        close_authority: false, // No close authority
        extensions,
    };
    let required_size = CompressedToken::byte_len(&config).map_err(ProgramError::from)?;
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

    // Use zero-copy new to initialize the token account
    let (mut compressed_token, _) = CompressedToken::new_zero_copy(&mut token_account_data, config)
        .map_err(|e| {
            msg!("Failed to create CompressedToken: {:?}", e);
            e
        })?;

    *compressed_token.mint = mint_pubkey.into();
    *compressed_token.owner = owner_pubkey.into();
    *compressed_token.state = 1; // Set state to Initialized
    if let Some(deref_compressible_config) = compressed_token.extensions.as_deref_mut() {
        let compressible_config =
            compressible_config.ok_or(ErrorCode::InvalidExtensionInstructionData)?;
        match deref_compressible_config.get_mut(0) {
            Some(ZExtensionStructMut::Compressible(compressible_extension)) => {
                let compressible_config_account =
                    compressible_config_account.ok_or(ErrorCode::InvalidCompressAuthority)?;
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
                compressible_extension.rent_authority =
                    compressible_config_account.rent_authority.to_bytes();
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
            }
            _ => {
                return Err(ErrorCode::InvalidExtensionInstructionData.into());
            }
        }
    }
    Ok(())
}
