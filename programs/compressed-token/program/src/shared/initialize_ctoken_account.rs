use anchor_lang::prelude::ProgramError;
use light_account_checks::AccountInfoTrait;
use light_compressible::config::CompressibleConfig;
use light_ctoken_interface::{
    instructions::create_ctoken_account::CompressToPubkey,
    state::{ctoken::CompressedTokenConfig, CToken},
    CTokenError, CTOKEN_PROGRAM_ID,
};
use light_program_profiler::profile;
use light_zero_copy::traits::ZeroCopyNew;
#[cfg(target_os = "solana")]
use pinocchio::sysvars::{clock::Clock, Sysvar};
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey};

use crate::extensions::MintExtensionFlags;

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

/// Compression-related instruction data for initializing a CToken account
#[derive(Debug, Clone, Copy)]
pub struct CompressionInstructionData {
    /// Version of the compressed token account when compressed
    pub token_account_version: u8,
    /// If true, the compressed token account cannot be transferred
    pub compression_only: u8,
    /// Write top-up in lamports per write
    pub write_top_up: u32,
}

/// Configuration for initializing a CToken account
pub struct CTokenInitConfig<'a> {
    /// The mint pubkey (32 bytes)
    pub mint: &'a [u8; 32],
    /// The owner pubkey (32 bytes)
    pub owner: &'a [u8; 32],
    /// Compression instruction data (all accounts now have compression fields embedded)
    pub compression_ix_data: CompressionInstructionData,
    /// Optional compress-to-pubkey configuration
    pub compress_to_pubkey: Option<&'a CompressToPubkey>,
    /// Compressible config account (if provided, compression is enabled)
    pub compressible_config_account: &'a CompressibleConfig,
    /// Custom rent payer pubkey (if not using default rent sponsor)
    pub custom_rent_payer: Option<Pubkey>,
    /// Mint extension flags
    pub mint_extensions: MintExtensionFlags,
    /// Mint account for caching decimals
    pub mint_account: &'a AccountInfo,
}

/// Initialize a token account using zero-copy with embedded CompressionInfo
#[profile]
pub fn initialize_ctoken_account(
    token_account_info: &AccountInfo,
    config: CTokenInitConfig<'_>,
) -> Result<(), ProgramError> {
    let CTokenInitConfig {
        mint,
        owner,
        compression_ix_data,
        compress_to_pubkey,
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

    // Build the config for new_zero_copy
    let zc_config = CompressedTokenConfig {
        mint: light_compressed_account::Pubkey::from(*mint),
        owner: light_compressed_account::Pubkey::from(*owner),
        state: if default_state_frozen { 2 } else { 1 },
        compression_only: compression_ix_data.compression_only != 0,
        has_pausable,
        has_permanent_delegate,
        has_transfer_fee,
        has_transfer_hook,
    };

    // Access the token account data as mutable bytes
    let mut token_account_data = AccountInfoTrait::try_borrow_mut_data(token_account_info)?;

    // Use new_zero_copy to initialize the token account
    // This sets mint, owner, state, compression_only, account_type, and extensions
    let (mut ctoken, _remaining) = CToken::new_zero_copy(&mut token_account_data, zc_config)
        .map_err(|e| {
            msg!("Failed to initialize CToken: {:?}", e);
            ProgramError::InvalidAccountData
        })?;

    // Configure compression info fields and decimals
    configure_compression_info(
        &mut ctoken.meta,
        compression_ix_data,
        compress_to_pubkey,
        compressible_config_account,
        custom_rent_payer,
        mint_account,
    )?;

    Ok(())
}

#[profile]
#[inline(always)]
fn configure_compression_info(
    meta: &mut light_ctoken_interface::state::ZCTokenZeroCopyMetaMut<'_>,
    compression_ix_data: CompressionInstructionData,
    compress_to_pubkey: Option<&CompressToPubkey>,
    compressible_config_account: &CompressibleConfig,
    custom_rent_payer: Option<Pubkey>,
    mint_account: &AccountInfo,
) -> Result<(), ProgramError> {
    // Set config_account_version
    meta.compression.config_account_version = compressible_config_account.version.into();

    #[cfg(target_os = "solana")]
    let current_slot = Clock::get()
        .map_err(|_| ProgramError::UnsupportedSysvar)?
        .slot;
    #[cfg(not(target_os = "solana"))]
    let current_slot = 1;
    meta.compression.last_claimed_slot = current_slot.into();

    // Initialize RentConfig from compressible config account
    meta.compression.rent_config.base_rent =
        compressible_config_account.rent_config.base_rent.into();
    meta.compression.rent_config.compression_cost = compressible_config_account
        .rent_config
        .compression_cost
        .into();
    meta.compression.rent_config.lamports_per_byte_per_epoch = compressible_config_account
        .rent_config
        .lamports_per_byte_per_epoch;
    meta.compression.rent_config.max_funded_epochs =
        compressible_config_account.rent_config.max_funded_epochs;
    meta.compression.rent_config.max_top_up =
        compressible_config_account.rent_config.max_top_up.into();

    // Set the compression_authority, rent_sponsor and lamports_per_write
    meta.compression.compression_authority =
        compressible_config_account.compression_authority.to_bytes();
    if let Some(custom_rent_payer) = custom_rent_payer {
        // The custom rent payer is the rent recipient.
        meta.compression.rent_sponsor = custom_rent_payer;
    } else {
        meta.compression.rent_sponsor = compressible_config_account.rent_sponsor.to_bytes();
    }

    // Validate write_top_up doesn't exceed max_top_up
    if compression_ix_data.write_top_up > compressible_config_account.rent_config.max_top_up as u32
    {
        msg!(
            "write_top_up {} exceeds max_top_up {}",
            compression_ix_data.write_top_up,
            compressible_config_account.rent_config.max_top_up
        );
        return Err(CTokenError::WriteTopUpExceedsMaximum.into());
    }
    meta.compression
        .lamports_per_write
        .set(compression_ix_data.write_top_up);
    meta.compression.compress_to_pubkey = compress_to_pubkey.is_some() as u8;

    // Validate token_account_version is ShaFlat (3)
    if compression_ix_data.token_account_version != 3 {
        msg!(
            "Invalid token_account_version: {}. Only version 3 (ShaFlat) is supported",
            compression_ix_data.token_account_version
        );
        return Err(ProgramError::InvalidInstructionData);
    }
    meta.compression.account_version = compression_ix_data.token_account_version;

    // Read decimals from mint account
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
        meta.set_decimals(mint_data[44]);
    }

    Ok(())
}
