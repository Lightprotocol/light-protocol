//! SDK helpers for compressing and decompressing PDAs.

pub mod compress_account;
pub mod compress_account_on_init;
pub mod compression_info;
pub mod config;
pub mod decompress_idempotent;

#[cfg(feature = "anchor")]
pub use compress_account::compress_account;
pub use compress_account::compress_pda_native;
#[cfg(feature = "anchor")]
pub use compress_account_on_init::{
    compress_account_on_init, prepare_accounts_for_compression_on_init,
};
pub use compress_account_on_init::{
    compress_account_on_init_native, prepare_accounts_for_compression_on_init_native,
};
pub use compression_info::{CompressAs, CompressionInfo, HasCompressionInfo};
pub use config::{
    process_initialize_compression_config_account_info,
    process_initialize_compression_config_checked, process_update_compression_config,
    CompressibleConfig, COMPRESSIBLE_CONFIG_SEED, MAX_ADDRESS_TREES_PER_SPACE,
};
pub use decompress_idempotent::prepare_accounts_for_decompress_idempotent;
