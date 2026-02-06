use anchor_lang::prelude::ProgramError;
use light_account_checks::AccountInfoTrait;
use light_compressible::config::CompressibleConfig;
use light_program_profiler::profile;
use light_token_interface::{
    instructions::extensions::CompressibleExtensionInstructionData,
    state::{
        token::TokenConfig, AccountState, CompressibleExtensionConfig, CompressionInfoConfig,
        ExtensionStructConfig, Token, ACCOUNT_TYPE_MINT,
    },
    TokenError, LIGHT_TOKEN_PROGRAM_ID,
};
use light_zero_copy::traits::ZeroCopyNew;
#[cfg(target_os = "solana")]
use pinocchio::sysvars::{clock::Clock, rent::Rent, Sysvar};
use pinocchio::{account_info::AccountInfo, instruction::Seed, msg, pubkey::Pubkey};

use crate::{
    extensions::MintExtensionFlags,
    shared::{convert_program_error, create_pda_account, transfer_lamports_via_cpi},
};

const SPL_TOKEN_ID: [u8; 32] = spl_token::ID.to_bytes();
const SPL_TOKEN_2022_ID: [u8; 32] = spl_token_2022::ID.to_bytes();

/// SPL Token Mint account length (82 bytes)
const SPL_MINT_LEN: usize = 82;
/// Token-2022 AccountType byte position
/// Token-2022 pads mints to BASE_ACCOUNT_LENGTH (165 bytes) before AccountType
/// Layout: 82 bytes mint data + 83 bytes padding + 1 byte AccountType
const T22_ACCOUNT_TYPE_OFFSET: usize = 165;

/// Configuration for compressible accounts
pub struct CompressibleInitData<'a> {
    /// Instruction data for compression settings
    pub ix_data: &'a CompressibleExtensionInstructionData,
    /// Compressible config account with rent and authority settings
    pub config_account: &'a CompressibleConfig,
    /// Custom rent payer pubkey (if not using default rent sponsor)
    pub custom_rent_payer: Option<Pubkey>,
    /// Whether this account is an ATA (determined by instruction path, not ix data)
    pub is_ata: bool,
    /// Rent exemption lamports paid at account creation (from Rent sysvar)
    pub rent_exemption_paid: u32,
}

/// Configuration for initializing a CToken account
pub struct CTokenInitConfig<'a> {
    /// The owner pubkey (32 bytes)
    pub owner: &'a [u8; 32],
    /// Compressible configuration (None = not compressible)
    pub compressible: Option<CompressibleInitData<'a>>,
    /// Mint extension flags
    pub mint_extensions: MintExtensionFlags,
    /// Mint account for caching decimals
    pub mint_account: &'a AccountInfo,
}

#[profile]
#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub fn create_compressible_account<'info>(
    compressible_config: &'info CompressibleExtensionInstructionData,
    mint_extensions: &MintExtensionFlags,
    config_account: &'info CompressibleConfig,
    rent_payer: &'info AccountInfo,
    target_account: &'info AccountInfo,
    fee_payer: &'info AccountInfo,
    account_seeds: Option<&[Seed]>,
    is_ata: bool,
) -> Result<CompressibleInitData<'info>, ProgramError> {
    // Validate rent payer is not the token account itself
    if rent_payer.key() == target_account.key() {
        msg!("Rent sponsor cannot be the token account itself");
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate rent_payment != 1 (epoch boundary edge case)
    if compressible_config.rent_payment == 1 {
        msg!("Prefunding for exactly 1 epoch is not allowed. If the account is created near an epoch boundary, it could become immediately compressible. Use 0 or 2+ epochs.");
        return Err(anchor_compressed_token::ErrorCode::OneEpochPrefundingNotAllowed.into());
    }

    // Calculate account size (includes Compressible extension)
    let account_size = mint_extensions.calculate_account_size(true)?;

    // Get rent exemption from Rent sysvar (only place we query it - store for later use)
    #[cfg(target_os = "solana")]
    let rent_exemption_paid: u32 = Rent::get()
        .map_err(|_| ProgramError::UnsupportedSysvar)?
        .minimum_balance(account_size as usize)
        .try_into()
        .map_err(|_| ProgramError::ArithmeticOverflow)?;
    #[cfg(not(target_os = "solana"))]
    let rent_exemption_paid = 0;

    // Calculate rent with compression cost
    let rent = config_account
        .rent_config
        .get_rent_with_compression_cost(account_size, compressible_config.rent_payment as u64);
    let account_size = account_size as usize;

    let custom_rent_payer = *rent_payer.key() != config_account.rent_sponsor.to_bytes();

    // Custom rent payer must be a signer (prevents executable accounts as rent_sponsor)
    if custom_rent_payer && !rent_payer.is_signer() {
        msg!("Custom rent payer must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Build rent sponsor seeds for PDA signing
    let version_bytes = config_account.version.to_le_bytes();
    let bump_seed = [config_account.rent_sponsor_bump];
    let rent_sponsor_seeds = [
        Seed::from(b"rent_sponsor".as_ref()),
        Seed::from(version_bytes.as_ref()),
        Seed::from(bump_seed.as_ref()),
    ];

    let fee_payer_seeds = if custom_rent_payer {
        None
    } else {
        Some(rent_sponsor_seeds.as_slice())
    };

    let additional_lamports = if custom_rent_payer { Some(rent) } else { None };

    // Create the account
    create_pda_account(
        rent_payer,
        target_account,
        account_size,
        fee_payer_seeds,
        account_seeds,
        additional_lamports,
    )?;

    // When using protocol rent sponsor, fee_payer pays the compression incentive
    if !custom_rent_payer {
        transfer_lamports_via_cpi(rent, fee_payer, target_account)
            .map_err(convert_program_error)?;
    }

    Ok(CompressibleInitData {
        ix_data: compressible_config,
        config_account,
        custom_rent_payer: if custom_rent_payer {
            Some(*rent_payer.key())
        } else {
            None
        },
        is_ata,
        rent_exemption_paid,
    })
}

/// Initialize a token account using zero-copy with embedded CompressionInfo
#[profile]
pub fn initialize_ctoken_account(
    token_account_info: &AccountInfo,
    config: CTokenInitConfig<'_>,
) -> Result<(), ProgramError> {
    let CTokenInitConfig {
        owner,
        compressible,
        mint_extensions,
        mint_account,
    } = config;

    // Build extensions Vec from boolean flags
    // +1 for potential Compressible extension
    let mut extensions = Vec::with_capacity(mint_extensions.num_token_account_extensions() + 1);
    // Add Compressible extension if compression is enabled
    if compressible.is_some() {
        extensions.push(ExtensionStructConfig::Compressible(
            CompressibleExtensionConfig {
                info: CompressionInfoConfig { rent_config: () },
            },
        ));

        if mint_extensions.has_pausable {
            extensions.push(ExtensionStructConfig::PausableAccount(()));
        }
        if mint_extensions.has_permanent_delegate {
            extensions.push(ExtensionStructConfig::PermanentDelegateAccount(()));
        }
        if mint_extensions.has_transfer_fee {
            extensions.push(ExtensionStructConfig::TransferFeeAccount(()));
        }
        if mint_extensions.has_transfer_hook {
            extensions.push(ExtensionStructConfig::TransferHookAccount(()));
        }
    } else if mint_extensions.has_restricted_extensions() {
        // Mints with restricted extensions must have the compressible extension.
        return Err(anchor_compressed_token::ErrorCode::MissingCompressibleConfig.into());
    }
    // Build the config for new_zero_copy
    let zc_config = TokenConfig {
        mint: light_compressed_account::Pubkey::from(*mint_account.key()),
        owner: light_compressed_account::Pubkey::from(*owner),
        state: if mint_extensions.default_state_frozen {
            AccountState::Frozen as u8
        } else {
            AccountState::Initialized as u8
        },
        extensions: if extensions.is_empty() {
            None
        } else {
            Some(extensions)
        },
    };

    // Access the token account data as mutable bytes
    let mut token_account_data = AccountInfoTrait::try_borrow_mut_data(token_account_info)?;

    // Use new_zero_copy to initialize the token account
    // This sets mint, owner, state, account_type, and extensions
    let (mut ctoken, _) =
        Token::new_zero_copy(&mut token_account_data, zc_config).map_err(|e| {
            msg!("Failed to initialize CToken: {:?}", e);
            ProgramError::InvalidAccountData
        })?;

    // Configure compression info fields only if compressible
    // We need to re-read using zero_copy_at_mut because new_zero_copy doesn't
    // populate the extensions field (it only writes them to bytes)
    if let Some(compressible) = compressible {
        configure_compression_info(&mut ctoken, compressible, mint_account)?;
    }

    Ok(())
}

#[profile]
#[inline(always)]
fn configure_compression_info(
    ctoken: &mut light_token_interface::state::ZTokenMut<'_>,
    compressible: CompressibleInitData<'_>,
    mint_account: &AccountInfo,
) -> Result<(), ProgramError> {
    let CompressibleInitData {
        ix_data,
        config_account,
        custom_rent_payer,
        is_ata,
        rent_exemption_paid,
    } = compressible;

    // Get the Compressible extension (must exist since we added it)
    let compressible_ext = ctoken
        .get_compressible_extension_mut()
        .ok_or(TokenError::MissingCompressibleExtension)?;

    // Set config_account_version
    compressible_ext.info.config_account_version = config_account.version.into();

    #[cfg(target_os = "solana")]
    let current_slot = Clock::get()
        .map_err(|_| ProgramError::UnsupportedSysvar)?
        .slot;
    #[cfg(not(target_os = "solana"))]
    let current_slot = 1;
    compressible_ext.info.last_claimed_slot = current_slot.into();

    // Initialize RentConfig from compressible config account
    compressible_ext.info.rent_config.base_rent = config_account.rent_config.base_rent.into();
    compressible_ext.info.rent_config.compression_cost =
        config_account.rent_config.compression_cost.into();
    compressible_ext
        .info
        .rent_config
        .lamports_per_byte_per_epoch = config_account.rent_config.lamports_per_byte_per_epoch;
    compressible_ext.info.rent_config.max_funded_epochs =
        config_account.rent_config.max_funded_epochs;
    compressible_ext.info.rent_config.max_top_up = config_account.rent_config.max_top_up.into();

    // Set rent exemption paid at account creation (store once, never query Rent sysvar again)
    compressible_ext.info.rent_exemption_paid = rent_exemption_paid.into();

    // Set the compression_authority, rent_sponsor and lamports_per_write
    compressible_ext.info.compression_authority = config_account.compression_authority.to_bytes();
    if let Some(custom_rent_payer) = custom_rent_payer {
        // The custom rent payer is the rent recipient.
        compressible_ext.info.rent_sponsor = custom_rent_payer;
    } else {
        compressible_ext.info.rent_sponsor = config_account.rent_sponsor.to_bytes();
    }

    // Validate write_top_up doesn't exceed max_top_up
    if ix_data.write_top_up > config_account.rent_config.max_top_up as u32 {
        msg!(
            "write_top_up {} exceeds max_top_up {}",
            ix_data.write_top_up,
            config_account.rent_config.max_top_up
        );
        return Err(TokenError::WriteTopUpExceedsMaximum.into());
    }
    compressible_ext
        .info
        .lamports_per_write
        .set(ix_data.write_top_up);
    compressible_ext.info.compress_to_pubkey = ix_data.compress_to_account_pubkey.is_some() as u8;

    // Set compression_only flag on the extension
    compressible_ext.compression_only = if ix_data.compression_only != 0 { 1 } else { 0 };

    // Set is_ata flag on the extension
    compressible_ext.is_ata = is_ata as u8;

    // Validate token_account_version is ShaFlat (3)
    if ix_data.token_account_version != 3 {
        msg!(
            "Invalid token_account_version: {}. Only version 3 (ShaFlat) is supported",
            ix_data.token_account_version
        );
        return Err(ProgramError::InvalidInstructionData);
    }
    compressible_ext.info.account_version = ix_data.token_account_version;

    // Read decimals from mint account and cache in extension
    let mint_data = AccountInfoTrait::try_borrow_data(mint_account)?;
    // Only try to read decimals if mint has data (is initialized)
    if !mint_data.is_empty() {
        let owner = mint_account.owner();

        if !is_valid_mint(owner, &mint_data)? {
            msg!("Invalid mint account: not a valid mint");
            return Err(ProgramError::InvalidAccountData);
        }

        // Mint layout: decimals at byte 44 for all token programs
        // (mint_authority option: 36, supply: 8) = 44
        compressible_ext.set_decimals(mint_data.get(44).copied());
    }

    Ok(())
}

#[inline(always)]
pub fn is_valid_mint(owner: &Pubkey, mint_data: &[u8]) -> Result<bool, ProgramError> {
    if *owner == SPL_TOKEN_ID {
        // SPL Token: mint must be exactly 82 bytes
        Ok(mint_data.len() == SPL_MINT_LEN)
    } else if *owner == SPL_TOKEN_2022_ID {
        // Token-2022: Either exactly 82 bytes (no extensions) or
        // check AccountType marker at offset 165 (with extensions)
        // Layout with extensions: 82 bytes mint + 83 bytes padding + AccountType
        Ok(mint_data.len() == SPL_MINT_LEN
            || (mint_data.len() > T22_ACCOUNT_TYPE_OFFSET
                && mint_data[T22_ACCOUNT_TYPE_OFFSET] == ACCOUNT_TYPE_MINT))
    } else if *owner == LIGHT_TOKEN_PROGRAM_ID {
        // CToken: Always has extensions, must be >165 bytes with AccountType=Mint
        Ok(mint_data.len() > T22_ACCOUNT_TYPE_OFFSET
            && mint_data[T22_ACCOUNT_TYPE_OFFSET] == ACCOUNT_TYPE_MINT)
    } else {
        msg!("Invalid mint owner");
        Err(ProgramError::IncorrectProgramId)
    }
}
