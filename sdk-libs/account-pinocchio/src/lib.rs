//! Light Protocol account types specialized for pinocchio's AccountInfo.

pub use pinocchio::account_info::AccountInfo;

// ===== TYPE ALIASES (structs generic over AI, specialized with pinocchio AccountInfo) =====
// Note: pinocchio's AccountInfo has no lifetime parameter, so aliases have fewer lifetimes.

pub type CpiAccounts<'c> = light_sdk_types::cpi_accounts::v2::CpiAccounts<'c, AccountInfo>;

pub type CpiAccountsV1<'c> = light_sdk_types::cpi_accounts::v1::CpiAccounts<'c, AccountInfo>;

pub type CompressCtx<'a> =
    light_sdk_types::interface::program::compression::processor::CompressCtx<'a, AccountInfo>;

pub type CompressDispatchFn =
    light_sdk_types::interface::program::compression::processor::CompressDispatchFn<AccountInfo>;

pub type DecompressCtx<'a> =
    light_sdk_types::interface::program::decompression::processor::DecompressCtx<'a, AccountInfo>;

pub type ValidatedPdaContext =
    light_sdk_types::interface::program::validation::ValidatedPdaContext<AccountInfo>;

pub type CpiContextWriteAccounts<'a> =
    light_sdk_types::cpi_context_write::CpiContextWriteAccounts<'a, AccountInfo>;

#[cfg(all(not(target_os = "solana"), feature = "std"))]
pub type PackedAccounts =
    light_sdk_types::interface::instruction::PackedAccounts<solana_instruction::AccountMeta>;

// ===== RE-EXPORTED TRAITS (generic over AI, used with explicit AccountInfo in impls) =====

pub use light_account_checks::close_account;
#[cfg(feature = "token")]
pub use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
// ===== RE-EXPORTED CONCRETE TRAITS (no AI parameter) =====
pub use light_sdk_types::interface::account::compression_info::{
    claim_completed_epoch_rent, CompressAs, CompressedAccountData, CompressedInitSpace,
    CompressionInfo, CompressionInfoField, CompressionState, HasCompressionInfo, Space,
    COMPRESSION_INFO_SIZE, OPTION_COMPRESSION_INFO_SPACE,
};
#[cfg(all(not(target_os = "solana"), feature = "std"))]
pub use light_sdk_types::interface::account::pack::Pack;
// ===== TOKEN-GATED RE-EXPORTS =====
#[cfg(feature = "token")]
pub use light_sdk_types::interface::account::token_seeds::{
    PackedTokenData, TokenDataWithPackedSeeds, TokenDataWithSeeds,
};
// Mint creation CPI types and functions
#[cfg(feature = "token")]
pub use light_sdk_types::interface::cpi::create_mints::{
    derive_mint_compressed_address as derive_mint_compressed_address_generic,
    get_output_queue_next_index, invoke_create_mints, CreateMintsCpi, CreateMintsInfraAccounts,
    CreateMintsParams, SingleMintParams, DEFAULT_RENT_PAYMENT, DEFAULT_WRITE_TOP_UP,
};
// Token account/ATA creation CPI types and functions
#[cfg(feature = "token")]
pub use light_sdk_types::interface::cpi::create_token_accounts::{
    derive_associated_token_account as derive_associated_token_account_generic,
    CreateTokenAccountCpi, CreateTokenAccountRentFreeCpi, CreateTokenAtaCpi,
    CreateTokenAtaCpiIdempotent, CreateTokenAtaRentFreeCpi,
};
// ===== RE-EXPORTED GENERIC FUNCTIONS (AI inferred from call-site args) =====
pub use light_sdk_types::interface::cpi::invoke::invoke_light_system_program;
#[cfg(feature = "token")]
pub use light_sdk_types::interface::program::decompression::processor::process_decompress_accounts_idempotent;
#[cfg(feature = "token")]
pub use light_sdk_types::interface::program::decompression::token::prepare_token_account_for_decompression;
#[cfg(feature = "token")]
pub use light_sdk_types::interface::program::variant::{PackedTokenSeeds, UnpackedTokenSeeds};
pub use light_sdk_types::interface::{
    account::{
        light_account::{AccountType, LightAccount},
        pack::Unpack,
        pda_seeds::{HasTokenVariant, PdaSeedDerivation},
    },
    accounts::{
        finalize::{LightFinalize, LightPreInit},
        init_compressed_account::{prepare_compressed_account_on_init, reimburse_rent},
    },
    cpi::{
        account::CpiAccountsTrait,
        invoke::{invoke_write_pdas_to_cpi_context, InvokeLightSystemProgram},
        LightCpi,
    },
    create_accounts_proof::CreateAccountsProof,
    program::{
        compression::{
            pda::prepare_account_for_compression,
            processor::{process_compress_pda_accounts_idempotent, CompressAndCloseParams},
        },
        config::{
            create::process_initialize_light_config, process_initialize_light_config_checked,
            process_update_light_config, InitializeLightConfigParams, LightConfig,
            UpdateLightConfigParams, LIGHT_CONFIG_SEED, MAX_ADDRESS_TREES_PER_SPACE,
        },
        decompression::{
            pda::prepare_account_for_decompression,
            processor::{
                process_decompress_pda_accounts_idempotent, DecompressIdempotentParams,
                DecompressVariant,
            },
        },
        validation::{
            extract_tail_accounts, is_pda_initialized, should_skip_compression,
            split_at_system_accounts_offset, validate_compress_accounts,
            validate_decompress_accounts,
        },
        variant::{IntoVariant, LightAccountVariantTrait, PackedLightAccountVariantTrait},
    },
    rent,
};
#[cfg(feature = "token")]
pub use light_token_interface::instructions::extensions::ExtensionInstructionData as TokenExtensionInstructionData;
// Token-interface re-exports for macro-generated code
#[cfg(feature = "token")]
pub use light_token_interface::instructions::extensions::TokenMetadataInstructionData;

#[cfg(feature = "token")]
pub mod token {
    pub use light_sdk_types::interface::{
        account::token_seeds::{
            ExtensionInstructionData, MultiInputTokenDataWithContext, PackedTokenData,
            TokenDataWithPackedSeeds, TokenDataWithSeeds,
        },
        program::decompression::token::prepare_token_account_for_decompression,
    };
}

pub mod compression_info {
    pub use light_sdk_types::interface::account::compression_info::*;
}

// ===== CPI / SDK-TYPES RE-EXPORTS =====

pub use light_sdk_types::cpi_accounts::CpiAccountsConfig;

#[cfg(all(not(target_os = "solana"), feature = "std"))]
pub mod interface {
    pub mod instruction {
        pub use light_sdk_types::interface::instruction::PackedAccounts;
    }
}

pub mod account_meta {
    pub use light_sdk_types::instruction::account_meta::*;
}

// ===== ACCOUNT-CHECKS RE-EXPORTS (used by macro-generated code) =====

pub extern crate light_account_checks;
// ===== CONVENIENCE RE-EXPORTS =====
pub use light_account_checks::{
    account_info::pinocchio::OwnedAccountMeta, discriminator::Discriminator as LightDiscriminator,
    packed_accounts, AccountInfoTrait, AccountMetaTrait,
};
pub use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
pub use light_macros::{derive_light_cpi_signer, derive_light_cpi_signer_pda};
pub use light_sdk_macros::{
    CompressAs, Compressible, HasCompressionInfo, LightAccount, LightDiscriminator, LightHasher,
    LightHasherSha, LightProgram,
};
pub use light_sdk_types::{constants, error::LightSdkTypesError, instruction::*, CpiSigner};

// ===== UTILITY FUNCTIONS =====

/// Converts a [`LightSdkTypesError`] into a [`pinocchio::program_error::ProgramError`].
///
/// Use with `.map_err(light_err)` in pinocchio instruction handlers to disambiguate
/// the multiple `From` implementations on `LightSdkTypesError`.
pub fn light_err(e: LightSdkTypesError) -> pinocchio::program_error::ProgramError {
    pinocchio::program_error::ProgramError::Custom(u32::from(e))
}

/// Derives the rent sponsor PDA for a given program.
///
/// Seeds: `["rent_sponsor"]`
/// Returns `([u8; 32], u8)` since pinocchio uses raw byte array pubkeys.
pub fn derive_rent_sponsor_pda(program_id: &[u8; 32]) -> ([u8; 32], u8) {
    <AccountInfo as AccountInfoTrait>::find_program_address(
        &[constants::RENT_SPONSOR_SEED],
        program_id,
    )
}

/// Find the mint PDA address for a given mint seed.
///
/// Returns `([u8; 32], u8)` -- the PDA address and bump.
#[cfg(feature = "token")]
pub fn find_mint_address(mint_seed: &[u8; 32]) -> ([u8; 32], u8) {
    light_sdk_types::interface::cpi::create_mints::find_mint_address::<AccountInfo>(mint_seed)
}

/// Derive the compressed mint address from a mint seed and address tree pubkey.
#[cfg(feature = "token")]
pub fn derive_mint_compressed_address(
    mint_seed: &[u8; 32],
    address_tree_pubkey: &[u8; 32],
) -> [u8; 32] {
    derive_mint_compressed_address_generic::<AccountInfo>(mint_seed, address_tree_pubkey)
}

/// Derive the associated token account address for a given owner and mint.
///
/// Returns `([u8; 32], u8)` -- the ATA address and bump seed.
#[cfg(feature = "token")]
pub fn derive_associated_token_account(owner: &[u8; 32], mint: &[u8; 32]) -> ([u8; 32], u8) {
    derive_associated_token_account_generic::<AccountInfo>(owner, mint)
}
