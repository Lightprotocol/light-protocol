//! Integration tests for freeze/thaw operations on compressed token accounts.
//!
//! Tests for freezing and thawing compressed token accounts using the anchor freeze instruction.

#![cfg(feature = "test-sbf")]

#[path = "freeze/compress_only.rs"]
mod compress_only;

#[path = "freeze/functional.rs"]
mod functional;
