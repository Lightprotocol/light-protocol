//! Unified seed classification and extraction for Light Protocol macros.
//!
//! This module provides:
//! - **Types**: `ClassifiedSeed`, `ClassifiedFnArg`, `FnArgKind`, `SeedSpec`
//! - **Classification**: `classify_seed_expr()` for classifying individual seeds
//! - **Extraction**: `extract_seed_specs()` for parsing Accounts structs
//! - **Anchor**: `extract_anchor_seeds()` for extracting seeds from #[account(...)] attributes
//! - **Data Fields**: `get_data_fields()`, `extract_data_field_info()` for data field extraction
//!
//! # Example
//!
//! ```ignore
//! use crate::light_pdas::seeds::{extract_seed_specs, SeedSpec, ClassifiedSeed};
//!
//! let specs = extract_seed_specs(&item_struct)?;
//! for spec in &specs {
//!     println!("Field: {}, Seeds: {}", spec.field_name, spec.seed_count());
//! }
//! ```

mod anchor_extraction;
mod classification;
mod data_fields;
mod extract;
mod instruction_args;
pub mod types;

// Re-export from data_fields
pub use data_fields::{
    extract_data_field_info, extract_data_field_name_from_expr, get_data_fields,
    get_params_only_seed_fields_from_spec,
};
// Re-export from extract
pub use extract::{extract_account_inner_type, extract_from_accounts_struct, extract_seed_specs};
// Re-export from instruction_args
pub use instruction_args::parse_instruction_arg_names;
// Re-export from types - public API
pub use types::{
    ClassifiedFnArg, ClassifiedSeed, ExtractedSeedSpec, ExtractedTokenSpec, FnArgKind, SeedSpec,
};
