//! Rent-free program macro implementation.
//!
//! This module provides `#[rentfree_program]` attribute macro that:
//! - Automatically discovers #[rentfree] fields in Accounts structs
//! - Auto-wraps instruction handlers with light_pre_init/light_finalize logic
//! - Generates all necessary types, enums, and instruction handlers

pub mod crate_context;
pub mod expr_traversal;
mod parsing;
mod decompress;
mod compress;
pub mod instructions;
pub mod seed_codegen;
pub mod seed_utils;
pub mod variant_enum;
pub mod visitors;

pub use instructions::rentfree_program_impl;
