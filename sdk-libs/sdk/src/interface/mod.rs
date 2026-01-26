pub mod v1;

// Explicitly re-export v1 items (modules)
pub use v1::{close, compression_info, config, finalize, traits};

// Re-export v1 items (types and functions)
pub use v1::{
    // finalize
    LightFinalize, LightPreInit,
    // traits
    IntoCTokenVariant, IntoVariant, PdaSeeds,
    // compression_info
    CompressAs, CompressedInitSpace, CompressionInfo, CompressionInfoField, CompressionState,
    HasCompressionInfo, Pack, PodCompressionInfoField, Space, Unpack, COMPRESSION_INFO_SIZE,
    OPTION_COMPRESSION_INFO_SPACE,
    // config
    process_initialize_light_config, process_initialize_light_config_checked,
    process_update_light_config, LightConfig, COMPRESSIBLE_CONFIG_SEED,
    MAX_ADDRESS_TREES_PER_SPACE,
    // light_compressible re-exports
    rent, CreateAccountsProof,
};

// v1 feature-gated re-exports
#[cfg(feature = "v2")]
pub use v1::{
    compress_account, compress_account_on_init, compress_runtime, decompress_idempotent,
    prepare_account_for_compression_pod,
    prepare_compressed_account_on_init_pod,
    CompressContext,
    compute_data_hash, create_pda_account, into_compressed_meta_with_address,
    prepare_account_for_decompression_idempotent, prepare_account_for_decompression_idempotent_pod,
};

#[cfg(all(feature = "v2", feature = "cpi-context"))]
pub use v1::{
    decompress_runtime,
    check_account_types, process_decompress_accounts_idempotent, DecompressContext,
    DecompressibleAccount, HasTokenVariant, PdaSeedDerivation, TokenSeedProvider,
};

// v2 module and exports (anchor feature required)
#[cfg(feature = "anchor")]
pub mod v2;

#[cfg(feature = "anchor")]
pub use v2::{
    // New traits from v2
    traits::{AccountType, LightAccount, LightAccountVariant, PackedLightAccountVariant},
    // Compress functions with v2 signatures
    compress::{
        prepare_account_for_compression, process_compress_pda_accounts_idempotent,
        CompressAndCloseParams, CompressCtx,
    },
    // Decompress functions with v2 signatures
    decompress::{
        prepare_account_for_decompression, process_decompress_pda_accounts_idempotent,
        DecompressCtx, DecompressIdempotentParams, DecompressVariant,
    },
    // Init function from v2
    init::prepare_compressed_account_on_init,
};
