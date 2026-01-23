//! Unit tests for shared utility functions.
//!
//! Extracted from `light_pdas/shared_utils.rs`.

use crate::light_pdas::shared_utils::is_constant_identifier;

#[test]
fn test_is_constant_identifier() {
    assert!(is_constant_identifier("MY_CONSTANT"));
    assert!(is_constant_identifier("SEED"));
    assert!(is_constant_identifier("SEED_123"));
    assert!(is_constant_identifier("A"));
    assert!(!is_constant_identifier("myVariable"));
    assert!(!is_constant_identifier("my_variable"));
    assert!(!is_constant_identifier("MyConstant"));
    assert!(!is_constant_identifier(""));
}
