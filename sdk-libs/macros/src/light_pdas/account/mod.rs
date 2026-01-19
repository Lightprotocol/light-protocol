//! Shared trait derive macros for compressible accounts.
//!
//! This module provides:
//! - `seed_extraction` - Seed extraction from Anchor account attributes
//! - `decompress_context` - Decompression context utilities
//! - `light_compressible` - Combined LightAccount derive macro
//! - `pack_unpack` - Pack/Unpack trait implementations
//! - `traits` - HasCompressionInfo, Compressible, CompressAs traits
//! - `utils` - Shared utility functions

pub mod decompress_context;
pub mod light_compressible;
pub mod pack_unpack;
pub mod seed_extraction;
#[allow(clippy::module_inception)]
pub mod traits;
pub mod utils;
