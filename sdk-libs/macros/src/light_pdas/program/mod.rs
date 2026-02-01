//! Light program macro implementation.
//!
//! This module provides `#[light_program]` attribute macro that:
//! - Automatically discovers #[light_account(init)] fields in Accounts structs
//! - Auto-wraps instruction handlers with light_pre_init/light_finalize logic
//! - Generates all necessary types, enums, and instruction handlers

pub(crate) mod compress;
mod decompress;
pub mod derive_light_program;
pub mod expr_traversal;
pub mod instructions;
pub mod seed_codegen;
pub mod seed_utils;
pub mod variant_enum;

// Made pub(crate) for testing in light_pdas_tests module
pub(crate) mod parsing;
pub(crate) mod visitors;

pub use derive_light_program::derive_light_program_impl;
pub use instructions::light_program_impl;
