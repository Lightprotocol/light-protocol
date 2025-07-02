use std::vec::Vec;

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::{borsh::Deserialize, ZeroCopyEq};
use light_zero_copy_derive::ZeroCopy;

// Simple struct with a primitive field and a vector
#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize, ZeroCopy, ZeroCopyEq)]
pub struct SimpleStruct {
    pub a: u8,
    pub b: Vec<u8>,
}

// Basic struct with all basic numeric types
#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize, ZeroCopy, ZeroCopyEq)]
pub struct NumericStruct {
    pub a: u8,
    pub b: u16,
    pub c: u32,
    pub d: u64,
    pub e: bool,
}

// use light_zero_copy::borsh_mut::DeserializeMut; // Not needed for non-mut derivations

#[test]
fn test_simple_from_implementation() {
    // Create an instance of our struct
    let original = SimpleStruct {
        a: 42,
        b: vec![1, 2, 3, 4, 5],
    };

    // Serialize it
    let bytes = original.try_to_vec().unwrap();
    // byte_len not available for non-mut derivations
    // assert_eq!(bytes.len(), original.byte_len());

    // Test From implementation for immutable struct
    let (zero_copy, _) = SimpleStruct::zero_copy_at(&bytes).unwrap();
    let converted: SimpleStruct = zero_copy.into();
    assert_eq!(converted.a, 42);
    assert_eq!(converted.b, vec![1, 2, 3, 4, 5]);
    assert_eq!(converted, original);
}

#[test]
fn test_numeric_from_implementation() {
    // Create a struct with different primitive types
    let original = NumericStruct {
        a: 1,
        b: 2,
        c: 3,
        d: 4,
        e: true,
    };

    // Serialize it
    let bytes = original.try_to_vec().unwrap();
    // byte_len not available for non-mut derivations
    // assert_eq!(bytes.len(), original.byte_len());

    // Test From implementation for immutable struct
    let (zero_copy, _) = NumericStruct::zero_copy_at(&bytes).unwrap();
    let converted: NumericStruct = zero_copy.clone().into();

    // Verify all fields
    assert_eq!(converted.a, 1);
    assert_eq!(converted.b, 2);
    assert_eq!(converted.c, 3);
    assert_eq!(converted.d, 4);
    assert!(converted.e);

    // Verify complete struct
    assert_eq!(converted, original);
}
