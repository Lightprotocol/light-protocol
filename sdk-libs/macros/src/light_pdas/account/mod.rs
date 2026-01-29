//! Shared trait derive macros for light accounts.
//!
//! This module provides:
//! - `derive` - Combined LightAccount derive macro
//! - `traits` - HasCompressionInfo, Compressible, CompressAs traits
//! - `utils` - Shared utility functions
//! - `validation` - Shared validation utilities

pub mod derive;
#[allow(clippy::module_inception)]
pub mod traits;
pub mod utils;
pub mod validation;
