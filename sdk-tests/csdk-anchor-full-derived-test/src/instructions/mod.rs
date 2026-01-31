//! Instruction account test cases organized by dimension.
//!
//! Each subdirectory tests a specific macro code path dimension:
//! - d5_markers: Field marker attributes (#[light_account(init)], #[light_account(token)], #[light_account(init)])
//! - d6_account_types: Account type extraction (Account, Box<Account>)
//! - d7_infra_names: Infrastructure field naming variations
//! - d8_builder_paths: Builder code generation paths
//! - d9_seeds: Seed expression classification
//! - d10_token_accounts: Token account and ATA creation via macro
//! - d11_zero_copy: Zero-copy (AccountLoader) tests

pub mod d10_token_accounts;
pub mod d11_zero_copy;
pub mod d5_markers;
pub mod d6_account_types;
pub mod d7_infra_names;
pub mod d8_builder_paths;
pub mod d9_seeds;
