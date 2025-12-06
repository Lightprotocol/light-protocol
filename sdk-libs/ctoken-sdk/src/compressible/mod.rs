//! Compressible token utilities for runtime decompression.

#[cfg(feature = "compressible")]
pub mod decompress_runtime;

#[cfg(feature = "compressible")]
pub use decompress_runtime::{process_decompress_tokens_runtime, CTokenSeedProvider};
