use anchor_lang::prelude::ProgramError;
use light_account_checks::AccountInfoTrait;
use light_compressible::config::CompressibleConfig;
use light_ctoken_interface::{
    instructions::extensions::compressible::CompressibleExtensionInstructionData,
    state::{calculate_ctoken_account_size, CompressibleExtension, ZCompressibleExtensionMut},
    CTokenError, CTOKEN_PROGRAM_ID,
};
use light_program_profiler::profile;
use light_zero_copy::traits::ZeroCopyAtMut;
#[cfg(target_os = "solana")]
use pinocchio::sysvars::{clock::Clock, Sysvar};
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey};

use crate::{extensions::MintExtensionFlags, ErrorCode};

const SPL_TOKEN_ID: [u8; 32] = spl_token::ID.to_bytes();
const SPL_TOKEN_2022_ID: [u8; 32] = spl_token_2022::ID.to_bytes();

/// SPL Token Mint account length (82 bytes)
const SPL_MINT_LEN: usize = 82;
/// Token-2022 AccountType byte position
/// Token-2022 pads mints to BASE_ACCOUNT_LENGTH (165 bytes) before AccountType
/// Layout: 82 bytes mint data + 83 bytes padding + 1 byte AccountType
const T22_ACCOUNT_TYPE_OFFSET: usize = 165;
/// AccountType::Mint discriminator value
const ACCOUNT_TYPE_MINT: u8 = 1;

/// Configuration for initializing a CToken account
pub struct CTokenInitConfig<'a> {
    /// The mint pubkey (32 bytes)
    pub mint: &'a [u8; 32],
    /// The owner pubkey (32 bytes)
    pub owner: &'a [u8; 32],
    /// Compressible extension instruction data (if compressible)
    pub compressible: Option<CompressibleExtensionInstructionData>,
    /// Compressible config account (required if compressible is Some)
    pub compressible_config_account: Option<&'a CompressibleConfig>,
    /// Custom rent payer pubkey (if not using default rent sponsor)
    pub custom_rent_payer: Option<Pubkey>,
    /// Mint extension flags
    pub mint_extensions: MintExtensionFlags,
    /// Mint account for caching decimals in compressible extension
    pub mint_account: &'a AccountInfo,
}

/// Initialize a token account using spl-pod with zero balance and default settings
#[profile]
pub fn initialize_ctoken_account(
    token_account_info: &AccountInfo,
    config: CTokenInitConfig<'_>,
) -> Result<(), ProgramError> {
    let CTokenInitConfig {
        mint,
        owner,
        compressible,
        compressible_config_account,
        custom_rent_payer,
        mint_extensions:
            MintExtensionFlags {
                has_pausable,
                has_permanent_delegate,
                default_state_frozen,
                has_transfer_fee,
                has_transfer_hook,
            },
        mint_account,
    } = config;

    let has_compressible = compressible.is_some();
    let required_size = calculate_ctoken_account_size(
        has_compressible,
        has_pausable,
        has_permanent_delegate,
        has_transfer_fee,
        has_transfer_hook,
    ) as usize;
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
    base_token_bytes[0..32].copy_from_slice(mint);

    // Copy owner (32 bytes at offset 32)
    base_token_bytes[32..64].copy_from_slice(owner);

    // Set state to Initialized (1) or Frozen (2) at offset 108
    // AccountState: Uninitialized = 0, Initialized = 1, Frozen = 2
    base_token_bytes[108] = if default_state_frozen { 2 } else { 1 };

    // Configure compressible extension if present
    if let Some(compressible_ix_data) = compressible {
        let compressible_config_account =
            compressible_config_account.ok_or(ErrorCode::InvalidCompressAuthority)?;
        // Split to get the actual CompressibleExtension data starting at byte 7
        // CompressibleExtension layout: 1 byte compression_only + CompressionInfo
        let (extension_bytes, compressible_data) = extension_bytes.split_at_mut(7);

        // Manually set extension metadata
        // Byte 0: AccountType::Account = 2
        extension_bytes[0] = 2;

        // Byte 1: Option::Some = 1 (for Option<Vec<ExtensionStruct>>)
        extension_bytes[1] = 1;

        // Bytes 2-5: Vec length (number of extensions)
        let mut extension_count = 1u32; // Always at least compressible
        if has_pausable {
            extension_count += 1;
        }
        if has_permanent_delegate {
            extension_count += 1;
        }
        if has_transfer_fee {
            extension_count += 1;
        }
        if has_transfer_hook {
            extension_count += 1;
        }
        extension_bytes[2..6].copy_from_slice(&extension_count.to_le_bytes());

        // Byte 6: Compressible enum discriminator = 32 (avoids Token-2022 overlap)
        extension_bytes[6] = 32;

        // Create zero-copy mutable reference to CompressibleExtension
        let (mut compressible_extension, remaining) =
            CompressibleExtension::zero_copy_at_mut(compressible_data).map_err(|e| {
                msg!(
                    "Failed to create CompressibleExtension zero-copy reference: {:?}",
                    e
                );
                ProgramError::InvalidAccountData
            })?;

        // Set compression_only field from instruction data
        compressible_extension.compression_only = compressible_ix_data.compression_only;

        configure_compressible_extension(
            &mut compressible_extension,
            compressible_ix_data,
            compressible_config_account,
            custom_rent_payer,
            mint_account,
        )?;

        // Add PausableAccount and PermanentDelegateAccount extensions if needed
        let mut remaining = remaining;

        if has_pausable {
            if remaining.is_empty() {
                msg!("Not enough space for PausableAccount extension");
                return Err(ErrorCode::InsufficientAccountSize.into());
            }
            let (pausable_bytes, rest) = remaining.split_at_mut(1);
            // Write PausableAccount discriminator (27)
            pausable_bytes[0] = 27;
            remaining = rest;
        }

        if has_permanent_delegate {
            if remaining.is_empty() {
                msg!("Not enough space for PermanentDelegateAccount extension");
                return Err(ErrorCode::InsufficientAccountSize.into());
            }
            let (permanent_delegate_bytes, rest) = remaining.split_at_mut(1);
            // Write PermanentDelegateAccount discriminator (28)
            permanent_delegate_bytes[0] = 28;
            remaining = rest;
        }

        if has_transfer_fee {
            if remaining.len() < 9 {
                msg!("Not enough space for TransferFeeAccount extension");
                return Err(ErrorCode::InsufficientAccountSize.into());
            }
            let (transfer_fee_bytes, rest) = remaining.split_at_mut(9);
            // Write TransferFeeAccount discriminator (29), withheld_amount already zeros
            transfer_fee_bytes[0] = 29;
            remaining = rest;
        }

        if has_transfer_hook {
            if remaining.len() < 2 {
                msg!("Not enough space for TransferHookAccount extension");
                return Err(ErrorCode::InsufficientAccountSize.into());
            }
            let (transfer_hook_bytes, _) = remaining.split_at_mut(2);
            // Write TransferHookAccount discriminator (30) + transferring flag (0)
            transfer_hook_bytes[0] = 30;
            transfer_hook_bytes[1] = 0; // transferring = false
        }
    }

    Ok(())
}

#[profile]
#[inline(always)]
fn configure_compressible_extension(
    compressible_extension: &mut ZCompressibleExtensionMut<'_>,
    compressible_ix_data: CompressibleExtensionInstructionData,
    compressible_config_account: &CompressibleConfig,
    custom_rent_payer: Option<Pubkey>,
    mint_account: &AccountInfo,
) -> Result<(), ProgramError> {
    // Set config_account_version
    compressible_extension.info.config_account_version = compressible_config_account.version.into();

    #[cfg(target_os = "solana")]
    let current_slot = Clock::get()
        .map_err(|_| ProgramError::UnsupportedSysvar)?
        .slot;
    #[cfg(not(target_os = "solana"))]
    let current_slot = 1;
    compressible_extension.info.last_claimed_slot = current_slot.into();
    // Initialize RentConfig with default values
    compressible_extension.info.rent_config.base_rent =
        compressible_config_account.rent_config.base_rent.into();
    compressible_extension.info.rent_config.compression_cost = compressible_config_account
        .rent_config
        .compression_cost
        .into();
    compressible_extension
        .info
        .rent_config
        .lamports_per_byte_per_epoch = compressible_config_account
        .rent_config
        .lamports_per_byte_per_epoch;
    compressible_extension.info.rent_config.max_funded_epochs =
        compressible_config_account.rent_config.max_funded_epochs;
    compressible_extension.info.rent_config.max_top_up =
        compressible_config_account.rent_config.max_top_up.into();

    // Set the compression_authority, rent_sponsor and lamports_per_write
    compressible_extension.info.compression_authority =
        compressible_config_account.compression_authority.to_bytes();
    if let Some(custom_rent_payer) = custom_rent_payer {
        // The custom rent payer is the rent recipient.
        // In this case the rent mechanism stay the same,
        // the account can be compressed and closed by a forester,
        // rent rewards cannot be claimed by the forester.
        compressible_extension.info.rent_sponsor = custom_rent_payer;
    } else {
        compressible_extension.info.rent_sponsor =
            compressible_config_account.rent_sponsor.to_bytes();
    }

    // Validate write_top_up doesn't exceed max_top_up
    if compressible_ix_data.write_top_up > compressible_config_account.rent_config.max_top_up as u32
    {
        msg!(
            "write_top_up {} exceeds max_top_up {}",
            compressible_ix_data.write_top_up,
            compressible_config_account.rent_config.max_top_up
        );
        return Err(CTokenError::WriteTopUpExceedsMaximum.into());
    }
    compressible_extension
        .info
        .lamports_per_write
        .set(compressible_ix_data.write_top_up);
    compressible_extension.info.compress_to_pubkey =
        compressible_ix_data.compress_to_account_pubkey.is_some() as u8;
    // Validate token_account_version is ShaFlat (3)
    if compressible_ix_data.token_account_version != 3 {
        msg!(
            "Invalid token_account_version: {}. Only version 3 (ShaFlat) is supported",
            compressible_ix_data.token_account_version
        );
        return Err(ProgramError::InvalidInstructionData);
    }
    compressible_extension.info.account_version = compressible_ix_data.token_account_version;

    let mint_data = AccountInfoTrait::try_borrow_data(mint_account)?;
    // Only try to read decimals if mint has data (is initialized)
    if !mint_data.is_empty() {
        let owner = mint_account.owner();

        // Validate mint account based on owner program
        let is_valid_mint = if *owner == SPL_TOKEN_ID {
            // SPL Token: mint must be exactly 82 bytes
            mint_data.len() == SPL_MINT_LEN
        } else if *owner == SPL_TOKEN_2022_ID || *owner == CTOKEN_PROGRAM_ID {
            // Token-2022/CToken: check AccountType marker at offset 165
            // Layout: 82 bytes mint + 83 bytes padding + AccountType
            mint_data.len() > T22_ACCOUNT_TYPE_OFFSET
                && mint_data[T22_ACCOUNT_TYPE_OFFSET] == ACCOUNT_TYPE_MINT
        } else {
            msg!("Invalid mint owner");
            return Err(ProgramError::IncorrectProgramId);
        };

        if !is_valid_mint {
            msg!("Invalid mint account: not a valid mint");
            return Err(ProgramError::InvalidAccountData);
        }

        // Mint layout: decimals at byte 44 for all token programs
        // (mint_authority option: 36, supply: 8) = 44
        // Already validated length above (SPL is 82 bytes, T22/CToken > 82 bytes)
        compressible_extension.set_decimals(mint_data[44]);
    }

    Ok(())
}
