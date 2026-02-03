//! Framework-agnostic interface for Light Protocol compressible accounts.

pub mod account;
pub mod accounts;
pub mod create_accounts_proof;
pub mod program;

// LightCpi trait + CPI builder (no runtime dep)
pub mod cpi;

// --- Re-exports from light-compressible ---
// =============================================================================
// FLAT RE-EXPORTS
// =============================================================================

// --- account/ ---
#[cfg(all(not(target_os = "solana"), feature = "std"))]
pub use account::pack::Pack;
// --- program/ ---
#[cfg(feature = "token")]
pub use account::token_seeds::{PackedTokenData, TokenDataWithPackedSeeds, TokenDataWithSeeds};
pub use account::{
    compression_info::{
        claim_completed_epoch_rent, CompressAs, CompressedAccountData, CompressedInitSpace,
        CompressionInfo, CompressionInfoField, CompressionState, HasCompressionInfo, Space,
        COMPRESSION_INFO_SIZE, OPTION_COMPRESSION_INFO_SPACE,
    },
    light_account::{AccountType, LightAccount},
    pack::Unpack,
    pda_seeds::{HasTokenVariant, PdaSeedDerivation},
};
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
// --- Re-exports ---
pub use create_accounts_proof::CreateAccountsProof;
pub use light_compressible::rent;
#[cfg(feature = "token")]
pub use program::decompression::processor::process_decompress_accounts_idempotent;
#[cfg(feature = "token")]
pub use program::decompression::token::prepare_token_account_for_decompression;
#[cfg(feature = "token")]
pub use program::variant::{PackedTokenSeeds, UnpackedTokenSeeds};
pub use program::{
    compression::{
        pda::prepare_account_for_compression,
        processor::{
            process_compress_pda_accounts_idempotent, CompressAndCloseParams, CompressCtx,
            CompressDispatchFn,
        },
    },
    config::{
        process_initialize_light_config_checked, process_update_light_config,
        InitializeLightConfigParams, LightConfig, UpdateLightConfigParams, LIGHT_CONFIG_SEED,
        MAX_ADDRESS_TREES_PER_SPACE,
    },
    decompression::{
        pda::prepare_account_for_decompression,
        processor::{
            process_decompress_pda_accounts_idempotent, DecompressCtx, DecompressIdempotentParams,
            DecompressVariant,
        },
    },
    validation::{
        extract_tail_accounts, is_pda_initialized, should_skip_compression,
        split_at_system_accounts_offset, validate_compress_accounts, validate_decompress_accounts,
        ValidatedPdaContext,
    },
    variant::{IntoVariant, LightAccountVariantTrait, PackedLightAccountVariantTrait},
};
