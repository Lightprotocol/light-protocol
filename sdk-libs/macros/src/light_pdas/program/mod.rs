//! Light program macro implementation.
//!
//! This module provides `#[light_program]` attribute macro that:
//! - Automatically discovers #[light_account(init)] fields in Accounts structs
//! - Auto-wraps instruction handlers with light_pre_init/light_finalize logic
//! - Generates all necessary types, enums, and instruction handlers

mod compress;
pub mod crate_context;
mod decompress;
pub mod expr_traversal;
pub mod instructions;
mod parsing;
pub mod seed_codegen;
pub mod seed_utils;
pub mod variant_enum;
pub mod visitors;

pub use instructions::light_program_impl;
