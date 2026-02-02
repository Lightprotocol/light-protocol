//! Framework-agnostic interface for Light Protocol compressible accounts.

pub mod account;
pub mod accounts;
pub mod create_accounts_proof;
pub mod program;

// LightCpi trait + CPI builder (no runtime dep)
pub mod cpi;

// Client-side instruction building (not available on Solana BPF)
#[cfg(not(target_os = "solana"))]
pub mod instruction;

// --- Re-exports from light-compressible ---
pub use light_compressible::rent;

// --- Re-exports ---
pub use create_accounts_proof::CreateAccountsProof;

// =============================================================================
// FLAT RE-EXPORTS
// =============================================================================

// --- account/ ---
pub use account::compression_info::{
    claim_completed_epoch_rent, CompressAs, CompressedAccountData, CompressedInitSpace,
    CompressionInfo, CompressionInfoField, CompressionState, HasCompressionInfo, Space,
    COMPRESSION_INFO_SIZE, OPTION_COMPRESSION_INFO_SPACE,
};
pub use account::light_account::{AccountType, LightAccount};
#[cfg(not(target_os = "solana"))]
pub use account::pack::Pack;
pub use account::pack::Unpack;
pub use account::pda_seeds::{HasTokenVariant, PdaSeedDerivation};

// --- accounts/ ---
pub use accounts::{
    finalize::{LightFinalize, LightPreInit},
    init_compressed_account::{prepare_compressed_account_on_init, reimburse_rent},
};

// --- cpi/ ---
pub use cpi::{
    account::CpiAccountsTrait,
    invoke::{invoke_light_system_program, InvokeLightSystemProgram},
    LightCpi,
};

// --- program/ ---
#[cfg(feature = "token")]
pub use account::token_seeds::{PackedTokenData, TokenDataWithPackedSeeds, TokenDataWithSeeds};
pub use program::compression::close::close;
pub use program::compression::pda::prepare_account_for_compression;
pub use program::compression::processor::{
    process_compress_pda_accounts_idempotent, CompressAndCloseParams, CompressCtx,
    CompressDispatchFn,
};
pub use program::config::{
    process_initialize_light_config_checked, process_update_light_config,
    InitializeLightConfigParams, LightConfig, UpdateLightConfigParams, COMPRESSIBLE_CONFIG_SEED,
    MAX_ADDRESS_TREES_PER_SPACE,
};
pub use program::decompression::pda::prepare_account_for_decompression;
#[cfg(feature = "token")]
pub use program::decompression::processor::process_decompress_accounts_idempotent;
pub use program::decompression::processor::{
    process_decompress_pda_accounts_idempotent, DecompressCtx, DecompressIdempotentParams,
    DecompressVariant,
};
#[cfg(feature = "token")]
pub use program::decompression::token::prepare_token_account_for_decompression;
pub use program::validation::{
    extract_tail_accounts, is_pda_initialized, should_skip_compression,
    split_at_system_accounts_offset, validate_compress_accounts, validate_decompress_accounts,
    ValidatedPdaContext,
};
pub use program::variant::IntoVariant;
pub use program::variant::{LightAccountVariantTrait, PackedLightAccountVariantTrait};
#[cfg(feature = "token")]
pub use program::variant::{PackedTokenSeeds, UnpackedTokenSeeds};
