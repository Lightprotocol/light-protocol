# Light-Zero-Copy-Derive

A procedural macro for deriving zero-copy deserialization for Rust structs used with Solana programs.

## Features

This crate provides two key derive macros:

1. `#[derive(ZeroCopy)]` - Implements zero-copy deserialization with:
   - The `zero_copy_at` and `zero_copy_at_mut` methods for deserialization
   - Full Borsh compatibility for serialization/deserialization
   - Efficient memory representation with no copying of data

2. `#[derive(ZeroCopyEq)]` - Adds equality comparison support:
   - Compare zero-copy instances with regular struct instances
   - Can be used alongside `ZeroCopy` for complete functionality
   - Derivation for Options<struct> is not robust and may not compile.

## Rules for Zero-Copy Deserialization

The macro follows these rules when generating code:

1. Creates a `ZStruct` for your struct that follows zero-copy principles
   1. Fields are extracted into a meta struct until reaching a `Vec`, `Option` or non-`Copy` type
   2. Vectors are represented as `ZeroCopySlice` and not included in the meta struct
   3. Integer types are replaced with their zerocopy equivalents (e.g., `u16` → `U16`)
   4. Fields after the first vector are directly included in the `ZStruct` and deserialized one by one
   5. If a vector contains a nested vector (non-`Copy` type), it must implement `Deserialize`
   6. Elements in an `Option` must implement `Deserialize`
   7. Types that don't implement `Copy` must implement `Deserialize` and are deserialized one by one

## Usage

### Basic Usage

```rust
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy_derive::ZeroCopy;

#[repr(C)]
#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize, ZeroCopy)]
pub struct MyStruct {
    pub a: u8,
    pub b: u16,
    pub vec: Vec<u8>,
    pub c: u64,
}
let my_struct = MyStruct {
    a: 1,
    b: 2,
    vec: vec![1u8; 32],
    c: 3,
};
// Use the struct with zero-copy deserialization
let mut bytes = my_struct.try_to_vec().unwrap();
let (zero_copy, _remaining) = MyStruct::zero_copy_at(&bytes).unwrap();
assert_eq!(zero_copy.a, 1);
let (mut zero_copy_mut, _remaining) = MyStruct::zero_copy_at_mut(&mut bytes).unwrap();
zero_copy_mut.a = 42;
let borsh = MyStruct::try_from_slice(&bytes).unwrap();
assert_eq!(borsh.a, 42u8);
```

### With Equality Comparison

```rust
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy_derive::ZeroCopy;

#[repr(C)]
#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize, ZeroCopy)]
pub struct MyStruct {
    pub a: u8,
    pub b: u16,
    pub vec: Vec<u8>,
    pub c: u64,
}
let my_struct = MyStruct {
    a: 1,
    b: 2,
    vec: vec![1u8; 32],
    c: 3,
};
// Use the struct with zero-copy deserialization
let mut bytes = my_struct.try_to_vec().unwrap();
let (zero_copy, _remaining) = MyStruct::zero_copy_at(&bytes).unwrap();
assert_eq!(zero_copy, my_struct);
```
