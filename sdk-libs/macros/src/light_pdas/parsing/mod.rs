//! Unified parsing module for Light Protocol macros.
//!
//! This module centralizes parsing logic for `#[derive(LightAccounts)]`.
//! It provides:
//!
//! - **InfraFields** - Infrastructure field classification by naming convention
//! - **InstructionArg** - Full instruction argument with type (for code generation)
//! - **InstructionArgSet** - Name-only set for seed classification
//! - **ParsedAccountsStruct** - Unified parsed Accounts struct
//! - **CrateContext** - Crate-wide module parsing for discovering structs

pub mod accounts_struct;
pub mod crate_context;
pub mod infra;
pub mod instruction_arg;

// Re-exports used by #[derive(LightAccounts)] via accounts/parse.rs
pub use accounts_struct::{ParsedAccountsStruct, ParsedPdaField};
// Re-export CrateContext for program-level discovery
pub use crate_context::CrateContext;
// Re-export parse_instruction_arg_names for seed classification
pub use instruction_arg::parse_instruction_arg_names;
