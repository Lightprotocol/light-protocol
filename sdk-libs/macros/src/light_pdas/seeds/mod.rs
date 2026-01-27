//! Seed classification and extraction for Light Protocol macros.
//!
//! This module provides:
//! - **Classification**: Via `classify_seed_expr()` from `account::seed_extraction`
//!   (unified classifier for both `#[derive(LightAccounts)]` and `#[light_program]`)
//! - **Types**: `SeedSpec` for per-field seed collections, `ClassifiedSeed` for individual seeds
//! - **Extraction**: `extract_seed_specs()` for parsing Accounts structs
//!
//! # Architecture
//!
//! All seed classification is done by `account::seed_extraction::classify_seed_expr()`.
//! This module (`seeds/`) provides the higher-level `SeedSpec` type and extraction logic
//! that wraps individual `ClassifiedSeed` values with field-level metadata.
//!
//! # Example
//!
//! ```ignore
//! use crate::light_pdas::seeds::{extract_seed_specs, SeedSpec};
//!
//! let specs = extract_seed_specs(&item_struct)?;
//! for spec in &specs {
//!     println!("Field: {}, Seeds: {}", spec.field_name, spec.seed_count());
//! }
//! ```

mod classify;
mod derive;
mod extract;
mod types;

// Re-export from extract
pub use extract::extract_seed_specs;

// Re-export from types
pub use types::SeedSpec;

// Re-export ClassifiedSeed from account::seed_extraction for convenience
pub use crate::light_pdas::account::seed_extraction::ClassifiedSeed;
