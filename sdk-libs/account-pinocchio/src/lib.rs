//! # Light Accounts Pinocchio
//!
//! Rent-free Light Accounts and Light Token Accounts for Pinocchio programs.
//!
//! ## How It Works
//!
//! **Light Accounts (PDAs)**
//! 1. Create a Solana PDA normally
//! 2. Register it with `#[derive(LightProgramPinocchio)]` - becomes a Light Account
//! 3. Use it as normal Solana account
//! 4. When rent runs out, account compresses (cold state)
//! 5. State preserved on-chain, client loads when needed (hot state)
//!
//! **Light Token Accounts (associated token accounts, Vaults)**
//! - Use `#[light_account(associated_token)]` for associated token accounts
//! - Use `#[light_account(token::seeds = [...], token::owner_seeds = [...])]` for vaults
//! - Cold/hot lifecycle
//!
//! **Light Mints**
//! - Created via `invoke_create_mints`
//! - Cold/hot lifecycle
//!
//! ## Quick Start
//!
//! ### 1. Program Setup
//!
//! ```rust,ignore
//! use light_account_pinocchio::{derive_light_cpi_signer, CpiSigner, LightProgramPinocchio};
//! use pinocchio_pubkey::pubkey;
//!
//! pub const ID: Pubkey = pubkey!("Your11111111111111111111111111111111111111");
//!
//! pub const LIGHT_CPI_SIGNER: CpiSigner =
//!     derive_light_cpi_signer!("Your11111111111111111111111111111111111111");
//! ```
//!
//! ### 2. State Definition
//!
//! ```rust,ignore
//! use borsh::{BorshDeserialize, BorshSerialize};
//! use light_account_pinocchio::{CompressionInfo, LightDiscriminator, LightHasherSha};
//!
//! #[derive(BorshSerialize, BorshDeserialize, LightDiscriminator, LightHasherSha)]
//! pub struct MyRecord {
//!     pub compression_info: CompressionInfo,  // Required first or last field
//!     pub owner: [u8; 32],
//!     pub data: u64,
//! }
//! ```
//!
//! ### 3. Program Accounts Enum
//!
//! ```rust,ignore
//! #[derive(LightProgramPinocchio)]
//! pub enum ProgramAccounts {
//!     #[light_account(pda::seeds = [b"record", ctx.owner])]
//!     MyRecord(MyRecord),
//! }
//! ```
//!
//! ## Account Types
//!
//! ### 1. Light Account (PDA)
//!
//! ```rust,ignore
//! #[light_account(pda::seeds = [b"record", ctx.owner])]
//! MyRecord(MyRecord),
//! ```
//!
//! ### 2. Light Account (zero-copy)
//!
//! ```rust,ignore
//! #[light_account(pda::seeds = [b"record", ctx.owner], pda::zero_copy)]
//! ZeroCopyRecord(ZeroCopyRecord),
//! ```
//!
//! ### 3. Light Token Account (vault)
//!
//! ```rust,ignore
//! #[light_account(token::seeds = [b"vault", ctx.mint], token::owner_seeds = [b"vault_auth"])]
//! Vault,
//! ```
//!
//! ### 4. Light Token Account (associated token account)
//!
//! ```rust,ignore
//! #[light_account(associated_token)]
//! Ata,
//! ```
//!
//! ## Required Derives
//!
//! | Derive | Use |
//! |--------|-----|
//! | `LightDiscriminator` | State structs (8-byte discriminator) |
//! | `LightHasherSha` | State structs (compression hashing) |
//! | `LightProgramPinocchio` | Program accounts enum |
//!
//! ## Required Macros
//!
//! | Macro | Use |
//! |-------|-----|
//! | `derive_light_cpi_signer!` | CPI signer PDA constant |
//! | `pinocchio_pubkey::pubkey!` | Program ID as `Pubkey` |
//!
//! For a complete example, see `sdk-tests/pinocchio-light-program-test`.

pub use pinocchio::account_info::AccountInfo;

// ===== TYPE ALIASES (structs generic over AI, specialized with pinocchio AccountInfo) =====
// Note: pinocchio's AccountInfo has no lifetime parameter, so aliases have fewer lifetimes.

pub type CpiAccounts<'c> = light_sdk_types::cpi_accounts::v2::CpiAccounts<'c, AccountInfo>;

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
    light_sdk_types::pack_accounts::PackedAccounts<solana_instruction::AccountMeta>;

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
    get_output_queue_next_index, CreateMints, CreateMintsCpi, CreateMintsParams,
    CreateMintsStaticAccounts, SingleMintParams, DEFAULT_RENT_PAYMENT, DEFAULT_WRITE_TOP_UP,
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
        size::Size,
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
        pub use light_sdk_types::pack_accounts::PackedAccounts;
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
pub use light_compressed_account::instruction_data::{
    compressed_proof::ValidityProof, cpi_context::CompressedCpiContext,
    with_account_info::InstructionDataInvokeCpiWithAccountInfo,
};
pub use light_macros::{derive_light_cpi_signer, derive_light_cpi_signer_pda, pubkey_array};
// Re-export for macro-generated client code (off-chain only)
#[cfg(feature = "std")]
pub extern crate solana_instruction;
#[cfg(feature = "std")]
pub extern crate solana_pubkey;
pub use light_sdk_macros::{
    AnchorDiscriminator as Discriminator, CompressAs, HasCompressionInfo, LightAccount,
    LightDiscriminator, LightHasher, LightHasherSha, LightPinocchioAccount, LightProgramPinocchio,
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
/// Returns `[u8; 32]` -- the ATA address.
#[cfg(feature = "token")]
pub fn derive_associated_token_account(owner: &[u8; 32], mint: &[u8; 32]) -> [u8; 32] {
    derive_associated_token_account_generic::<AccountInfo>(owner, mint)
}
