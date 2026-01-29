//! Parsing logic for #[light_account(...)] attributes.
//!
//! This module handles struct-level parsing and field classification.
//! The unified #[light_account] attribute parsing is in `light_account.rs`.
//!
//! This module now delegates to the unified parsing module, using type aliases
//! for backwards compatibility with existing code generation.

use syn::{DeriveInput, Error};

// Re-export unified types as type aliases for backwards compatibility
pub(super) type ParsedLightAccountsStruct = crate::light_pdas::parsing::ParsedAccountsStruct;
pub(super) type ParsedPdaField = crate::light_pdas::parsing::ParsedPdaField;

// Import infrastructure field types from unified parsing module
pub(super) use crate::light_pdas::parsing::infra::{InfraFieldType, InfraFields};
// Import instruction arg types from unified parsing module
pub(super) use crate::light_pdas::parsing::instruction_arg::InstructionArg;

// ============================================================================
// Main Parsing Function
// ============================================================================

/// Parse a struct to extract light_account fields (PDAs and mints).
///
/// Delegates to the unified parsing module.
pub(super) fn parse_light_accounts_struct(
    input: &DeriveInput,
) -> Result<ParsedLightAccountsStruct, Error> {
    crate::light_pdas::parsing::accounts_struct::parse_derive_input(input)
}
