//! Rent-free account compression macros.
//!
//! This module organizes all rent-free related macros:
//! - `program/` - `#[rentfree_program]` attribute macro for program-level auto-discovery
//! - `accounts/` - `#[derive(RentFree)]` derive macro for Accounts structs
//! - `account/` - Trait derive macros for account data structs (Compressible, Pack, HasCompressionInfo, etc.)
//! - `shared_utils` - Common utilities (constant detection, identifier extraction)

pub mod account;
pub mod accounts;
pub mod program;
pub mod shared_utils;
