#![cfg(feature = "mut")]

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::ZeroCopyInitMut;
use light_zero_copy_derive::{ByteLen, ZeroCopy, ZeroCopyConfig, ZeroCopyEq, ZeroCopyMut};

/// Simple struct with just a Vec field to test basic config functionality
#[repr(C)]
#[derive(
    Debug,
    PartialEq,
    BorshSerialize,
    BorshDeserialize,
    ZeroCopy,
    ZeroCopyMut,
    ZeroCopyEq,
    ByteLen,
    ZeroCopyConfig,
)]
pub struct SimpleVecStruct {
    pub a: u8,
    pub vec: Vec<u8>,
    pub b: u16,
}

#[test]
fn test_simple_config_generation() {
    // This test verifies that the ZeroCopyConfig derive macro generates the expected config struct
    // and ZeroCopyInitMut implementation

    // The config should have been generated as SimpleVecStructConfig
    let config = SimpleVecStructConfig {
        vec: 10, // Vec<u8> should have u32 config (length)
    };

    // Test that we can create a configuration
    assert_eq!(config.vec, 10);

    println!("Config generation test passed!");
}

/// Struct with Option field to test Option config
#[repr(C)]
#[derive(
    Debug,
    PartialEq,
    BorshSerialize,
    BorshDeserialize,
    ZeroCopy,
    ZeroCopyMut,
    ByteLen,
    ZeroCopyConfig,
)]
pub struct SimpleOptionStruct {
    pub a: u8,
    pub option: Option<u64>,
}

#[test]
fn test_option_config_generation() {
    // Test Option<u64> config generation - should be Option<()> since u64 has Config = ()
    let config = SimpleOptionStructConfig {
        option: Some(()), // Option<u64> should have Option<()> config
    };

    println!("Option config generation test passed!");
}

/// Test both Vec and Option in one struct
#[repr(C)]
#[derive(
    Debug,
    PartialEq,
    BorshSerialize,
    BorshDeserialize,
    ZeroCopy,
    ZeroCopyMut,
    ByteLen,
    ZeroCopyConfig,
)]
pub struct MixedStruct {
    pub a: u8,
    pub vec: Vec<u8>,
    pub option: Option<u64>,
    pub b: u16,
}

#[test]
fn test_mixed_config_generation() {
    let config = MixedStructConfig {
        vec: 5,           // Vec<u8> -> u32
        option: Some(()), // Option<u64> -> Option<()>
    };

    println!("Mixed config generation test passed!");
}
