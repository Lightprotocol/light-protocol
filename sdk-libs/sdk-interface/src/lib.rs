//! Framework-agnostic interface for Light Protocol compressible accounts.
//!
//! This crate provides the interface for compressible accounts, organized by
//! macro hierarchy:
//!
//! - `program/` - #[light_program] level (instruction processors)
//! - `accounts/` - #[derive(LightAccounts)] level (context structs, validation)
//! - `account/` - #[derive(LightAccount)] level (single account operations)
//!
//! # Features
//! - `solana` (default) - Enables Solana runtime support
//! - `pinocchio` - Enables Pinocchio runtime support
//! - `anchor` - Enables Anchor framework support
//! - `v2` - Enables v2 Light system program instructions
//! - `cpi-context` - Enables CPI context operations

// --- Conditional serialization trait aliases ---
#[cfg(feature = "anchor")]
pub use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
pub use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};

pub mod error;

// --- Subdirectory modules ---
pub mod account;
pub mod accounts;
pub mod program;

// --- CPI module (solana-only for now) ---
#[cfg(feature = "solana")]
pub mod cpi;

// --- Instruction module ---
#[cfg(feature = "solana")]
pub mod instruction;

// --- Re-exports from light-account-checks ---
pub use light_account_checks::{self, discriminator::Discriminator as LightDiscriminator};

// =============================================================================
// BACKWARD COMPATIBILITY: Submodule path preservation
// =============================================================================

/// Re-export config module for backward compatibility.
pub mod config {
    pub use super::program::config::*;
}

/// Re-export validation module for backward compatibility.
pub mod validation {
    pub use super::program::validation::*;
}

/// Re-export token module for backward compatibility.
#[cfg(feature = "anchor")]
pub mod token {
    pub use super::{
        account::token_seeds::*,
        program::decompression::token::prepare_token_account_for_decompression,
    };
}

/// Re-export compression_info module for backward compatibility.
pub mod compression_info {
    pub use super::account::compression_info::*;
}

/// Re-export close module for backward compatibility.
pub mod close {
    pub use super::program::compression::close::*;
}

/// Re-export finalize module for backward compatibility.
pub mod finalize {
    pub use super::accounts::finalize::*;
}

/// Re-export traits module for backward compatibility.
pub mod traits {
    #[cfg(feature = "anchor")]
    pub use super::account::light_account::{AccountType, LightAccount};
    pub use super::program::variant::IntoVariant;
    #[cfg(feature = "anchor")]
    pub use super::program::variant::{
        LightAccountVariantTrait, PackedLightAccountVariantTrait, PackedTokenSeeds,
        UnpackedTokenSeeds,
    };
}

// =============================================================================
// FLAT RE-EXPORTS
// =============================================================================

// --- Re-exports from account/ ---
#[cfg(feature = "anchor")]
pub use account::light_account::{AccountType, LightAccount};
#[cfg(all(not(target_os = "solana"), feature = "solana"))]
pub use account::pack::Pack;
pub use account::{
    compression_info::{
        claim_completed_epoch_rent, CompressAs, CompressedAccountData, CompressedInitSpace,
        CompressionInfo, CompressionInfoField, CompressionState, HasCompressionInfo, Space,
        COMPRESSION_INFO_SIZE, OPTION_COMPRESSION_INFO_SPACE,
    },
    pack::Unpack,
};
pub use account::pda_seeds::{HasTokenVariant, PdaSeedDerivation};
// --- Re-exports from accounts/ ---
pub use accounts::create_pda::create_pda_account;
pub use accounts::{
    finalize::{LightFinalize, LightPreInit},
    init_compressed_account::{
        prepare_compressed_account_on_init, prepare_compressed_account_on_init_checked,
        reimburse_rent,
    },
};
// --- Re-exports from external crates ---
pub use light_compressible::{rent, CreateAccountsProof};
pub use program::compression::close::close;
#[cfg(feature = "anchor")]
pub use program::compression::pda::prepare_account_for_compression;
#[cfg(feature = "anchor")]
pub use program::compression::processor::process_compress_pda_accounts_idempotent;
#[cfg(feature = "anchor")]
pub use program::compression::processor::{CompressAndCloseParams, CompressCtx, CompressDispatchFn};
#[cfg(feature = "anchor")]
pub use program::decompression::pda::prepare_account_for_decompression;
#[cfg(feature = "anchor")]
pub use program::decompression::processor::{
    process_decompress_pda_accounts_idempotent, DecompressCtx, DecompressIdempotentParams,
    DecompressVariant,
};
#[cfg(feature = "anchor")]
pub use program::variant::{
    LightAccountVariantTrait, PackedLightAccountVariantTrait, PackedTokenSeeds, UnpackedTokenSeeds,
};
pub use program::{
    config::{
        process_initialize_light_config, process_initialize_light_config_checked,
        process_update_light_config, LightConfig, COMPRESSIBLE_CONFIG_SEED,
        MAX_ADDRESS_TREES_PER_SPACE,
    },
    validation::{
        extract_tail_accounts, is_pda_initialized, should_skip_compression,
        split_at_system_accounts_offset, validate_compress_accounts, validate_decompress_accounts,
        ValidatedPdaContext,
    },
    variant::IntoVariant,
};
