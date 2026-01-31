//! Property-based, fuzz, and unit tests for light_pdas module.
//!
//! This module contains comprehensive tests for:
//! - Seed classification (`prop_tests.rs`, `fuzz_tests.rs`)
//! - Shared utilities (`shared_utils_prop_tests.rs`, `shared_utils_tests.rs`)
//! - Keyword validation (`keywords_prop_tests.rs`)
//! - Accounts parsing (`parse_prop_tests.rs`, `parsing_tests.rs`)
//! - Crate context (`crate_context_tests.rs`)

// Property-based and fuzz tests
mod fuzz_tests;
mod keywords_prop_tests;
mod parse_prop_tests;
mod prop_tests;
mod shared_utils_prop_tests;

// Unit tests extracted from source files
mod crate_context_tests;
mod light_compressible_tests;
mod parsing_tests;
mod shared_utils_tests;
mod visitors_tests;
