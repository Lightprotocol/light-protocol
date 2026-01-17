//! Rent-free account compression macros.
//!
//! This module organizes all rent-free related macros:
//! - `program/` - `#[rentfree_program]` attribute macro for program-level auto-discovery
//! - `accounts/` - `#[derive(RentFree)]` derive macro for Accounts structs
//! - `traits/` - Shared trait derive macros (Compressible, Pack, HasCompressionInfo, etc.)
//! - `shared_utils` - Common utilities (constant detection, identifier extraction)

pub mod accounts;
pub mod program;
pub mod shared_utils;
pub mod traits;
