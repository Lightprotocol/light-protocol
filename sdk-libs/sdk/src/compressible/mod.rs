//! SDK helpers for compressing and decompressing compressible PDAs accounts.

pub mod compress_account;
pub mod compress_account_on_init;
pub mod compress_account_on_init_native;
pub mod compression_info;
pub mod config;
pub mod decompress_idempotent;

#[cfg(feature = "anchor")]
pub use compress_account::compress_account;
#[cfg(feature = "v2")]
pub use compress_account::compress_pda_native;
#[cfg(all(feature = "anchor", feature = "v2"))]
pub use compress_account_on_init::{
    compress_account_on_init, compress_empty_account_on_init,
    prepare_accounts_for_compression_on_init, prepare_empty_compressed_accounts_on_init,
};
#[cfg(feature = "v2")]
pub use compress_account_on_init_native::{
    compress_account_on_init_native, compress_empty_account_on_init_native,
    prepare_accounts_for_compression_on_init_native,
    prepare_empty_compressed_accounts_on_init_native,
};
pub use compression_info::{CompressAs, CompressionInfo, HasCompressionInfo, Pack, Unpack};
pub use config::{
    process_initialize_compression_config_account_info,
    process_initialize_compression_config_checked, process_update_compression_config,
    CompressibleConfig, COMPRESSIBLE_CONFIG_SEED, MAX_ADDRESS_TREES_PER_SPACE,
};
pub use decompress_idempotent::into_compressed_meta_with_address;
#[cfg(feature = "v2")]
pub use decompress_idempotent::prepare_account_for_decompression_idempotent;
