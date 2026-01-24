//! Property-based, fuzz, and unit tests for light_pdas module.
//!
//! This module contains comprehensive tests for:
//! - Seed extraction and classification (`prop_tests.rs`, `seed_extraction_tests.rs`)
//! - Shared utilities (`shared_utils_prop_tests.rs`, `shared_utils_tests.rs`)
//! - Keyword validation (`keywords_prop_tests.rs`, `light_account_keywords_tests.rs`)
//! - Accounts parsing (`parse_prop_tests.rs`, `parsing_tests.rs`)
//! - E2E derive macro (`e2e_prop_tests.rs`)
//! - Fuzz tests (`fuzz_tests.rs`)
//! - Unit tests extracted from source files

// Property-based and fuzz tests
mod e2e_prop_tests;
mod fuzz_tests;
mod keywords_prop_tests;
mod parse_prop_tests;
mod prop_tests;
mod shared_utils_prop_tests;

// Unit tests extracted from source files
mod crate_context_tests;
mod derive_tests;
mod light_account_keywords_tests;
mod light_account_tests;
mod light_compressible_tests;
mod parsing_tests;
mod seed_extraction_tests;
mod shared_utils_tests;
mod visitors_tests;
