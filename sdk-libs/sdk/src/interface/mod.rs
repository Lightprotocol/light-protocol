//! Light Protocol interface module.
//!
//! This module provides the interface for compressible accounts, organized by
//! macro hierarchy:
//!
//! - `program/` - #[light_program] level (instruction processors)
//! - `accounts/` - #[derive(LightAccounts)] level (context structs, validation)
//! - `account/` - #[derive(LightAccount)] level (single account operations)

// --- Subdirectory modules ---
pub mod account;
pub mod accounts;
pub mod program;

// =============================================================================
// BACKWARD COMPATIBILITY: Submodule path preservation
// =============================================================================
// External code uses paths like `light_sdk::interface::config::LightConfig`
// and `light_sdk::interface::token::*`. Preserve with re-export aliases.

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
#[cfg(feature = "v2")]
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
    #[cfg(feature = "anchor")]
    pub use super::program::variant::{
        LightAccountVariantTrait, PackedLightAccountVariantTrait, PackedTokenSeeds,
        UnpackedTokenSeeds,
    };
    pub use super::{account::pda_seeds::PdaSeeds, program::variant::IntoVariant};
}

// =============================================================================
// BACKWARD COMPATIBILITY: Flat re-exports at interface level
// =============================================================================
// The root interface/mod.rs re-exports everything at the flat level for
// backward compatibility with existing code.

// --- Re-exports from program/ ---
// --- Re-exports from account/ ---
// Pack trait is only available off-chain (client-side) - uses PackedAccounts
#[cfg(feature = "anchor")]
pub use account::light_account::{AccountType, LightAccount};
#[cfg(not(target_os = "solana"))]
pub use account::pack::Pack;
// --- Re-exports from program/variant ---
pub use account::pda_seeds::PdaSeeds;
#[cfg(all(feature = "v2", feature = "cpi-context"))]
pub use account::pda_seeds::{HasTokenVariant, PdaSeedDerivation};
pub use account::{
    compression_info::{
        claim_completed_epoch_rent, CompressAs, CompressedAccountData, CompressedInitSpace,
        CompressionInfo, CompressionInfoField, CompressionState, HasCompressionInfo, Space,
        COMPRESSION_INFO_SIZE, OPTION_COMPRESSION_INFO_SPACE,
    },
    pack::Unpack,
};
// --- Re-exports from accounts/ ---
#[cfg(feature = "v2")]
pub use accounts::create_pda::create_pda_account;
pub use accounts::finalize::{LightFinalize, LightPreInit};
pub use accounts::init_compressed_account::{
    prepare_compressed_account_on_init, prepare_compressed_account_on_init_checked, reimburse_rent,
};
// --- Re-exports from external crates ---
pub use light_compressible::{rent, CreateAccountsProof};
#[cfg(feature = "v2")]
pub use program::compression::close::close;
#[cfg(feature = "anchor")]
pub use program::compression::pda::prepare_account_for_compression;
#[cfg(feature = "anchor")]
pub use program::compression::processor::process_compress_pda_accounts_idempotent;
#[cfg(feature = "anchor")]
pub use program::compression::processor::{
    CompressAndCloseParams, CompressCtx, CompressDispatchFn,
};
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
