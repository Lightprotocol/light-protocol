//! Unified seed classification and extraction for Light Protocol macros.
//!
//! This module provides:
//! - **Types**: `ClassifiedSeed`, `ClassifiedFnArg`, `FnArgKind`
//! - **Classification**: `classify_seed_expr()` for classifying individual seeds
//! - **Anchor**: `extract_anchor_seeds()` for extracting seeds from #[account(...)] attributes
//! - **Data Fields**: `get_data_fields()`, `extract_data_field_info()` for data field extraction
//! - **InstructionArgSet**: Canonical type for instruction argument name tracking
//!
//! # Relationship with `parsing/` Module
//!
//! The `parsing/` module provides unified struct parsing and re-exports `InstructionArgSet`
//! from this module. The classification types (`ClassifiedSeed`, etc.) remain here as the
//! canonical location for seed classification logic.

pub(crate) mod anchor_extraction;
pub(crate) mod classification;
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
pub use extract::{extract_account_inner_type, extract_from_accounts_struct};
// Re-export from instruction_args
pub use instruction_args::InstructionArgSet;
// Re-export from types - public API
pub use types::{
    ClassifiedFnArg, ClassifiedSeed, ExtractedSeedSpec, ExtractedTokenSpec, FnArgKind,
};
