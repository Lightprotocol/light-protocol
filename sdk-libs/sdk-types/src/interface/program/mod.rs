//! Program-level interface functions for #[light_program].
//!
//! This module contains functions used by the `#[light_program]` macro for
//! compress/decompress instruction processing.

pub mod compression;
pub mod config;
pub mod decompression;
pub mod validation;
pub mod variant;
