//! Instruction account test cases organized by dimension.
//!
//! Each subdirectory tests a specific macro code path dimension:
//! - d5_markers: Field marker attributes (#[rentfree], #[rentfree_token], #[light_mint])
//! - d6_account_types: Account type extraction (Account, Box<Account>)
//! - d7_infra_names: Infrastructure field naming variations
//! - d8_builder_paths: Builder code generation paths
//! - d9_seeds: Seed expression classification

pub mod d5_markers;
