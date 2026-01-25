//! Rent-free account compression macros.
//!
//! This module organizes all rent-free related macros:
//! - `program/` - `#[light_program]` attribute macro for program-level auto-discovery
//! - `accounts/` - `#[derive(LightAccounts)]` derive macro for Accounts structs
//! - `account/` - Trait derive macros for account data structs (Compressible, Pack, HasCompressionInfo, etc.)
//! - `seeds/` - Simplified seed extraction and classification (3-category system)
//! - `light_account_keywords` - Shared keyword definitions for `#[light_account(...)]` parsing
//! - `shared_utils` - Common utilities (constant detection, identifier extraction)

pub mod account;
pub mod accounts;
pub mod light_account_keywords;
pub mod program;
pub mod seeds;
pub mod shared_utils;
