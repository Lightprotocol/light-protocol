//! Shared trait derive macros for light accounts.
//!
//! This module provides:
//! - `light_compressible` - Combined LightAccount derive macro
//! - `traits` - HasCompressionInfo, Compressible, CompressAs traits
//! - `utils` - Shared utility functions
//! - `validation` - Shared validation utilities

pub mod light_compressible;
#[allow(clippy::module_inception)]
pub mod traits;
pub mod utils;
pub mod validation;
