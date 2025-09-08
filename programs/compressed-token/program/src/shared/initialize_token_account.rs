use anchor_lang::prelude::ProgramError;
use light_account_checks::AccountInfoTrait;
use light_ctoken_types::{
    instructions::extensions::compressible::ZCompressibleExtensionInstructionData,
    state::{
        CompressedToken, CompressedTokenConfig, CompressibleExtensionConfig, ExtensionStructConfig,
        ZExtensionStructMut,
    },
};
use light_zero_copy::ZeroCopyNew;
#[cfg(target_os = "solana")]
use pinocchio::sysvars::{clock::Clock, Sysvar};
use pinocchio::{account_info::AccountInfo, msg};

use crate::ErrorCode;

/// Initialize a token account using spl-pod with zero balance and default settings
pub fn initialize_token_account(
    token_account_info: &AccountInfo,
    mint_pubkey: &[u8; 32],
    owner_pubkey: &[u8; 32],
    compressible_config: Option<ZCompressibleExtensionInstructionData>,
    rent_paid: Option<u64>,
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
    let extensions = if let Some(compressible_config) = compressible_config.as_ref() {
        if compressible_config.has_rent_authority != 1 {
            msg!("Ctoken account with compressible extension must have rent authority and rent recipient");
            return Err(ProgramError::InvalidInstructionData);
        }
        if compressible_config.has_rent_authority != compressible_config.has_rent_recipient {
            msg!("Ctoken account with compressible extension must have rent authority and rent recipient");
            return Err(ProgramError::InvalidInstructionData);
        }
        vec![ExtensionStructConfig::Compressible(
            CompressibleExtensionConfig {
                rent_authority: (compressible_config.has_rent_authority != 0, ()),
                rent_recipient: (compressible_config.has_rent_recipient != 0, ()),
                write_top_up_lamports: compressible_config.has_top_up != 0,
            },
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
                // Set version to 1 (initialized)
                compressible_extension.version = 1;

                #[cfg(target_os = "solana")]
                let current_slot = Clock::get()
                    .map_err(|_| ProgramError::UnsupportedSysvar)?
                    .slot;
                #[cfg(not(target_os = "solana"))]
                let current_slot = 1;
                *compressible_extension.last_claimed_slot = current_slot.into();
                *compressible_extension.lamports_at_last_claimed_slot =
                    (current_lamports - rent_paid.unwrap()).into();
                if let Some(rent_authority) = compressible_extension.rent_authority.as_deref_mut() {
                    *rent_authority = compressible_config.rent_authority.to_bytes();
                }
                if let Some(rent_recipient) = compressible_extension.rent_recipient.as_deref_mut() {
                    *rent_recipient = compressible_config.rent_recipient.to_bytes();
                }
                if let Some(write_top_up_lamports) =
                    compressible_extension.write_top_up_lamports.as_deref_mut()
                {
                    *write_top_up_lamports = compressible_config.write_top_up;
                }
            }
            _ => {
                return Err(ErrorCode::InvalidExtensionInstructionData.into());
            }
        }
    }
    Ok(())
}
