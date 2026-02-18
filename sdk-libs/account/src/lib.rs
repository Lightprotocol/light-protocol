//! # Light Accounts
//!
//! Rent-free Light Accounts and Light Token Accounts for Anchor programs.
//!
//! ## How It Works
//!
//! **Light Accounts (PDAs)**
//! 1. Create a Solana PDA normally (Anchor `init`)
//! 2. Add `#[light_account(init)]` - becomes a Light Account
//! 3. Use it as normal Solana account
//! 4. When rent runs out, account compresses (cold state)
//! 5. State preserved on-chain, client loads when needed (hot state)
//! 6. When account is hot, use it as normal Solana account
//!
//! **Light Token Accounts (associated token accounts, Vaults)**
//! - Use `#[light_account(init, associated_token, ...)]` for associated token accounts
//! - Use `#[light_account(init, token, ...)]` for program-owned vaults
//! - Cold/hot lifecycle
//!
//! **Light Mints**
//! - Created via `CreateMintsCpi`
//! - Cold/hot lifecycle
//!
//! ## Quick Start
//!
//! ### 1. Program Setup
//!
//! ```rust,ignore
//! use light_account::{derive_light_cpi_signer, light_program, CpiSigner};
//!
//! declare_id!("Your11111111111111111111111111111111111111");
//!
//! pub const LIGHT_CPI_SIGNER: CpiSigner =
//!     derive_light_cpi_signer!("Your11111111111111111111111111111111111111");
//!
//! #[light_program]
//! #[program]
//! pub mod my_program {
//!     // ...
//! }
//! ```
//!
//! ### 2. State Definition
//!
//! ```rust,ignore
//! use light_account::{CompressionInfo, LightAccount};
//!
//! #[derive(Default, LightAccount)]
//! #[account]
//! pub struct UserRecord {
//!     pub compression_info: CompressionInfo,  // Required field
//!     pub owner: Pubkey,
//!     pub data: u64,
//! }
//! ```
//!
//! ### 3. Accounts Struct
//!
//! ```rust,ignore
//! use light_account::{CreateAccountsProof, LightAccounts};
//!
//! #[derive(AnchorSerialize, AnchorDeserialize)]
//! pub struct CreateParams {
//!     pub create_accounts_proof: CreateAccountsProof,
//!     pub owner: Pubkey,
//! }
//!
//! #[derive(Accounts, LightAccounts)]
//! #[instruction(params: CreateParams)]
//! pub struct CreateRecord<'info> {
//!     #[account(mut)]
//!     pub fee_payer: Signer<'info>,
//!
//!     /// CHECK: Compression config
//!     pub compression_config: AccountInfo<'info>,
//!
//!     /// CHECK: Rent sponsor
//!     #[account(mut)]
//!     pub pda_rent_sponsor: AccountInfo<'info>,
//!
//!     #[account(init, payer = fee_payer, space = 8 + UserRecord::INIT_SPACE, seeds = [b"record", params.owner.as_ref()], bump)]
//!     #[light_account(init)]
//!     pub record: Account<'info, UserRecord>,
//!
//!     pub system_program: Program<'info, System>,
//! }
//! ```
//!
//! ## Account Types
//!
//! ### 1. Light Account (PDA)
//!
//! ```rust,ignore
//! #[account(init, payer = fee_payer, space = 8 + MyRecord::INIT_SPACE, seeds = [...], bump)]
//! #[light_account(init)]
//! pub record: Account<'info, MyRecord>,
//! ```
//!
//! ### 2. Light Account (zero-copy)
//!
//! ```rust,ignore
//! #[account(init, payer = fee_payer, space = 8 + size_of::<MyZcRecord>(), seeds = [...], bump)]
//! #[light_account(init, zero_copy)]
//! pub record: AccountLoader<'info, MyZcRecord>,
//! ```
//!
//! ### 3. Light Token Account (vault)
//!
//! **With `init` (Anchor-created):**
//! ```rust,ignore
//! #[account(mut, seeds = [b"vault", mint.key().as_ref()], bump)]
//! #[light_account(init, token::seeds = [b"vault", self.mint.key()], token::owner_seeds = [b"vault_authority"])]
//! pub vault: UncheckedAccount<'info>,
//! ```
//!
//! **Without `init` (manual creation via `CreateTokenAccountCpi`):**
//! ```rust,ignore
//! #[account(mut, seeds = [b"vault", mint.key().as_ref()], bump)]
//! #[light_account(token::seeds = [b"vault", self.mint.key()], token::owner_seeds = [b"vault_authority"])]
//! pub vault: UncheckedAccount<'info>,
//! ```
//!
//! ### 4. Light Token Account (associated token account)
//!
//! **With `init` (Anchor-created):**
//! ```rust,ignore
//! #[account(mut)]
//! #[light_account(init, associated_token::authority = owner, associated_token::mint = mint)]
//! pub token_account: UncheckedAccount<'info>,
//! ```
//!
//! **Without `init` (manual creation via `CreateTokenAtaCpi`):**
//! ```rust,ignore
//! #[account(mut)]
//! #[light_account(associated_token::authority = owner, associated_token::mint = mint)]
//! pub token_account: UncheckedAccount<'info>,
//! ```
//!
//! ### 5. Light Mint
//!
//! ```rust,ignore
//! #[account(mut)]
//! #[light_account(init,
//!     mint::signer = mint_signer,           // PDA that signs mint creation
//!     mint::authority = mint_authority,     // Mint authority
//!     mint::decimals = 9,                   // Token decimals
//!     mint::seeds = &[SEED, self.key.as_ref()],  // Seeds for mint PDA
//!     mint::bump = params.bump,             // Bump seed
//!     // Optional: PDA authority
//!     mint::authority_seeds = &[b"authority"],
//!     mint::authority_bump = params.auth_bump,
//!     // Optional: Token metadata
//!     mint::name = params.name,
//!     mint::symbol = params.symbol,
//!     mint::uri = params.uri,
//!     mint::update_authority = update_auth,
//!     mint::additional_metadata = params.metadata
//! )]
//! pub mint: UncheckedAccount<'info>,
//! ```
//!
//! ## Required Derives
//!
//! | Derive | Use |
//! |--------|-----|
//! | `LightAccount` | State structs (must have `compression_info: CompressionInfo`) |
//! | `LightAccounts` | Accounts structs with `#[light_account(...)]` fields |
//!
//! ## Required Macros
//!
//! | Macro | Use |
//! |-------|-----|
//! | `#[light_program]` | Program module (before `#[program]`) |
//! | `derive_light_cpi_signer!` | CPI signer PDA constant |
//! | `derive_light_rent_sponsor_pda!` | Rent sponsor PDA (optional) |

pub use solana_account_info::AccountInfo;

// ===== TYPE ALIASES (structs generic over AI, specialized with AccountInfo) =====

pub type CpiAccounts<'c, 'info> =
    light_sdk_types::cpi_accounts::v2::CpiAccounts<'c, AccountInfo<'info>>;

pub type CompressCtx<'a, 'info> =
    light_sdk_types::interface::program::compression::processor::CompressCtx<
        'a,
        AccountInfo<'info>,
    >;

pub type CompressDispatchFn<'info> =
    light_sdk_types::interface::program::compression::processor::CompressDispatchFn<
        AccountInfo<'info>,
    >;

pub type DecompressCtx<'a, 'info> =
    light_sdk_types::interface::program::decompression::processor::DecompressCtx<
        'a,
        AccountInfo<'info>,
    >;

pub type ValidatedPdaContext<'info> =
    light_sdk_types::interface::program::validation::ValidatedPdaContext<AccountInfo<'info>>;

#[cfg(not(target_os = "solana"))]
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
#[cfg(not(target_os = "solana"))]
pub use light_sdk_types::interface::account::pack::Pack;
// ===== TOKEN-GATED RE-EXPORTS =====
#[cfg(feature = "token")]
pub use light_sdk_types::interface::account::token_seeds::{
    PackedTokenData, TokenDataWithPackedSeeds, TokenDataWithSeeds,
};
// create_accounts SDK function and parameter types
#[cfg(feature = "light-account")]
pub use light_sdk_types::interface::accounts::create_accounts::{
    create_accounts, AtaInitParam, CreateMintsInput, PdaInitParam, SharedAccounts, TokenInitParam,
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
            process_initialize_light_config_checked, process_update_light_config,
            InitializeLightConfigParams, LightConfig, UpdateLightConfigParams, LIGHT_CONFIG_SEED,
            MAX_ADDRESS_TREES_PER_SPACE,
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
pub use light_token_interface::state::AdditionalMetadata;
/// Re-export Token state struct for client-side use.
#[cfg(feature = "token")]
pub use light_token_interface::state::{AccountState, Token};

/// Token sub-module for paths like `light_account::token::TokenDataWithSeeds`.
#[cfg(feature = "token")]
pub mod token {
    pub use light_sdk_types::interface::{
        account::token_seeds::{
            ExtensionInstructionData, MultiInputTokenDataWithContext, PackedTokenData,
            TokenDataWithPackedSeeds, TokenDataWithSeeds,
        },
        program::decompression::token::prepare_token_account_for_decompression,
    };
    pub use light_token_interface::state::{AccountState, Token};
}

/// Compression info sub-module for paths like `light_account::compression_info::CompressedInitSpace`.
pub mod compression_info {
    pub use light_sdk_types::interface::account::compression_info::*;
}

// ===== CPI / SDK-TYPES RE-EXPORTS =====

pub use light_sdk_types::{
    cpi_accounts::CpiAccountsConfig, cpi_context_write::CpiContextWriteAccounts,
    interface::program::config::create::process_initialize_light_config,
};

/// Sub-module for generic `PackedAccounts<AM>` (not specialized to AccountMeta).
#[cfg(not(target_os = "solana"))]
pub mod interface {
    pub mod instruction {
        pub use light_sdk_types::pack_accounts::PackedAccounts;
    }
}

/// Sub-module for account_meta types (e.g. `CompressedAccountMetaNoLamportsNoAddress`).
pub mod account_meta {
    pub use light_sdk_types::instruction::account_meta::*;
}

// ===== ACCOUNT-CHECKS RE-EXPORTS (used by macro-generated code) =====

/// Re-export `light_account_checks` so consumers can use `light_account::light_account_checks::*`.
pub extern crate light_account_checks;
// ===== CONVENIENCE RE-EXPORTS =====
pub use light_account_checks::{
    discriminator::Discriminator as LightDiscriminator, packed_accounts, AccountInfoTrait,
    AccountMetaTrait,
};
pub use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
pub use light_compressible::rent::RentConfig;
pub use light_macros::{derive_light_cpi_signer, derive_light_cpi_signer_pda};
pub use light_sdk_macros::{
    // Attribute macros
    account,
    // Proc macros
    derive_light_rent_sponsor,
    derive_light_rent_sponsor_pda,
    light_program,
    // Derive macros
    AnchorDiscriminator as Discriminator,
    CompressAs,
    HasCompressionInfo,
    LightAccount,
    LightAccounts,
    LightDiscriminator,
    LightHasher,
    LightHasherSha,
    LightProgram,
};
pub use light_sdk_types::{
    constants,
    constants::{CPI_AUTHORITY_PDA_SEED, RENT_SPONSOR_SEED},
    error::LightSdkTypesError,
    instruction::*,
    interface::account::size::Size,
    CpiSigner,
};

/// Hasher re-exports for macro-generated code paths like `light_account::hasher::DataHasher`.
pub mod hasher {
    pub use light_hasher::{errors::HasherError, DataHasher, Hasher};
}

/// Re-export LIGHT_TOKEN_PROGRAM_ID as Pubkey for Anchor's `#[account(address = ...)]`.
pub const LIGHT_TOKEN_PROGRAM_ID: solana_pubkey::Pubkey =
    solana_pubkey::Pubkey::new_from_array(constants::LIGHT_TOKEN_PROGRAM_ID);

/// Default compressible config PDA for the Light Token Program.
pub const LIGHT_TOKEN_CONFIG: solana_pubkey::Pubkey =
    solana_pubkey::Pubkey::new_from_array(constants::LIGHT_TOKEN_CONFIG);

/// Default rent sponsor PDA for the Light Token Program.
pub const LIGHT_TOKEN_RENT_SPONSOR: solana_pubkey::Pubkey =
    solana_pubkey::Pubkey::new_from_array(constants::LIGHT_TOKEN_RENT_SPONSOR);

// ===== UTILITY FUNCTIONS =====

/// Converts a [`LightSdkTypesError`] into an [`anchor_lang::error::Error`].
///
/// Use with `.map_err(light_err)` in Anchor instruction handlers to disambiguate
/// the multiple `From` implementations on `LightSdkTypesError`.
#[cfg(feature = "anchor")]
pub fn light_err(e: LightSdkTypesError) -> anchor_lang::error::Error {
    anchor_lang::error::Error::from(e)
}

/// Derives the rent sponsor PDA for a given program.
///
/// Seeds: `["rent_sponsor"]`
pub fn derive_rent_sponsor_pda(program_id: &solana_pubkey::Pubkey) -> (solana_pubkey::Pubkey, u8) {
    solana_pubkey::Pubkey::find_program_address(&[constants::RENT_SPONSOR_SEED], program_id)
}

/// Find the mint PDA address for a given mint seed.
///
/// Returns `([u8; 32], u8)` -- the PDA address and bump.
#[cfg(feature = "token")]
pub fn find_mint_address(mint_seed: &[u8; 32]) -> ([u8; 32], u8) {
    light_sdk_types::interface::cpi::create_mints::find_mint_address::<AccountInfo<'static>>(
        mint_seed,
    )
}

/// Derive the compressed mint address from a mint seed and address tree pubkey.
#[cfg(feature = "token")]
pub fn derive_mint_compressed_address(
    mint_seed: &[u8; 32],
    address_tree_pubkey: &[u8; 32],
) -> [u8; 32] {
    derive_mint_compressed_address_generic::<AccountInfo<'static>>(mint_seed, address_tree_pubkey)
}

/// Derive the associated token account address for a given owner and mint.
///
/// Returns the ATA address.
#[cfg(feature = "token")]
pub fn derive_associated_token_account(
    owner: &solana_pubkey::Pubkey,
    mint: &solana_pubkey::Pubkey,
) -> solana_pubkey::Pubkey {
    let bytes = derive_associated_token_account_generic::<AccountInfo<'static>>(
        &owner.to_bytes(),
        &mint.to_bytes(),
    );
    solana_pubkey::Pubkey::from(bytes)
}
