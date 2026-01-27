// --- Always-available modules ---
pub mod close;
pub mod compression_info;
pub mod config;
pub mod finalize;
pub mod traits;

// --- v2-feature-gated modules ---
#[cfg(feature = "v2")]
pub mod decompress_idempotent;
#[cfg(all(feature = "v2", feature = "cpi-context"))]
pub mod decompress_runtime;

// --- anchor-feature-gated modules ---
#[cfg(feature = "anchor")]
pub mod compress;
#[cfg(feature = "anchor")]
pub mod decompress;
#[cfg(feature = "anchor")]
pub mod init;

// --- Always-available re-exports ---
// --- v2-feature-gated re-exports ---
#[cfg(feature = "v2")]
pub use close::close;
// --- anchor-feature-gated re-exports ---
#[cfg(feature = "anchor")]
pub use compress::{
    prepare_account_for_compression, process_compress_pda_accounts_idempotent,
    CompressAndCloseParams, CompressCtx,
};
pub use compression_info::{
    CompressAs, CompressedInitSpace, CompressionInfo, CompressionInfoField, CompressionState,
    HasCompressionInfo, Pack, PodCompressionInfoField, Space, Unpack, COMPRESSION_INFO_SIZE,
    OPTION_COMPRESSION_INFO_SPACE,
};
pub use config::{
    process_initialize_light_config, process_initialize_light_config_checked,
    process_update_light_config, LightConfig, COMPRESSIBLE_CONFIG_SEED,
    MAX_ADDRESS_TREES_PER_SPACE,
};
#[cfg(feature = "anchor")]
pub use decompress::{
    prepare_account_for_decompression, process_decompress_pda_accounts_idempotent, DecompressCtx,
    DecompressIdempotentParams, DecompressVariant,
};
#[cfg(feature = "v2")]
pub use decompress_idempotent::create_pda_account;
#[cfg(all(feature = "v2", feature = "cpi-context"))]
pub use decompress_runtime::{HasTokenVariant, PdaSeedDerivation, TokenSeedProvider};
pub use finalize::{LightFinalize, LightPreInit};
#[cfg(feature = "anchor")]
pub use init::prepare_compressed_account_on_init;
pub use light_compressible::{rent, CreateAccountsProof};
#[cfg(feature = "anchor")]
pub use traits::{
    AccountType, LightAccount, LightAccountVariantTrait, PackedLightAccountVariantTrait,
};
pub use traits::{IntoCTokenVariant, IntoVariant, PdaSeeds};
