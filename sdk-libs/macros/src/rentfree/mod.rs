//! Rent-free account compression macros.
//!
//! This module organizes all rent-free related macros:
//! - `program/` - `#[rentfree_program]` attribute macro for program-level auto-discovery
//! - `accounts/` - `#[derive(RentFree)]` derive macro for Accounts structs
//! - `traits/` - Shared trait derive macros (Compressible, Pack, HasCompressionInfo, etc.)

pub mod accounts;
pub mod program;
pub mod traits;
