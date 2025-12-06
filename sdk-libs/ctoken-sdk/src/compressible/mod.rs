//! Compressible token utilities for runtime decompression.

#[cfg(feature = "compressible")]
mod decompress_runtime;

#[cfg(feature = "compressible")]
pub use decompress_runtime::*;
