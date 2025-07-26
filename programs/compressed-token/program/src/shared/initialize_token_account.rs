use anchor_lang::prelude::ProgramError;
use light_account_checks::AccountInfoTrait;
use light_ctoken_types::{
    instructions::extensions::compressible::ZCompressibleExtensionInstructionData,
    state::{CompressedToken, CompressedTokenConfig, ExtensionStructConfig, ZExtensionStructMut},
};
use light_zero_copy::init_mut::ZeroCopyNew;
use pinocchio::{account_info::AccountInfo, msg, sysvars::clock::Clock};

/// Initialize a token account using spl-pod with zero balance and default settings
pub fn initialize_token_account(
    token_account_info: &AccountInfo,
    mint_pubkey: &[u8; 32],
    owner_pubkey: &[u8; 32],
    compressible_config: Option<ZCompressibleExtensionInstructionData>,
) -> Result<(), ProgramError> {
    // Access the token account data as mutable bytes
    let mut token_account_data = AccountInfoTrait::try_borrow_mut_data(token_account_info)
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Create configuration for the compressed token
    let extensions = if compressible_config.is_some() {
        vec![ExtensionStructConfig::Compressible]
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
    let required_size = CompressedToken::byte_len(&config);
    let actual_size = token_account_data.len();

    // Check account size before attempting to initialize
    if actual_size < required_size {
        msg!(
            "Account too small: required {} bytes, got {} bytes",
            required_size,
            actual_size
        );
        return Err(ProgramError::InvalidAccountData);
    }
    msg!("config {:?}", config);

    // Use zero-copy new to initialize the token account
    let (mut compressed_token, _) = CompressedToken::new_zero_copy(&mut token_account_data, config)
        .map_err(|e| {
            msg!("Failed to create CompressedToken: {:?}", e);
            ProgramError::InvalidAccountData
        })?;
    *compressed_token.mint = mint_pubkey.into();
    *compressed_token.owner = owner_pubkey.into();
    *compressed_token.state = 1; // Set state to Initialized
    if let Some(deref_compressible_config) = compressed_token.extensions.as_deref_mut() {
        msg!("compressible_config {:?}", compressible_config);
        let compressible_config =
            compressible_config.ok_or(ProgramError::InvalidInstructionData)?;
        msg!("deref_compressible_config {:?}", deref_compressible_config);
        match deref_compressible_config.get_mut(0) {
            Some(ZExtensionStructMut::Compressible(compressible_extension)) => {
                msg!("Compressible {:?}", compressible_extension);

                use pinocchio::sysvars::Sysvar;
                let current_slot = Clock::get().unwrap().slot;
                compressible_extension.last_written_slot = current_slot.into();
                compressible_extension.rent_authority = compressible_config.rent_authority;
                compressible_extension.rent_recipient = compressible_config.rent_recipient;
                compressible_extension.slots_until_compression =
                    compressible_config.slots_until_compression;
            }
            _ => {
                return Err(ProgramError::InvalidInstructionData);
            }
        }
    }
    Ok(())
}
