//! Procedural macros for borsh compatible zero copy serialization.
//!
//!
//!
//! ## Main Macros
//!
//! - `ZeroCopy`: Derives ZeroCopyAt
//! - `ZeroCopyMut`: Derives ZeroCopyAtMut, ZeroCopyNew
//! - `ZeroCopyEq`: Derives PartialEq for <StructName>::ZeroCopy == StructName
//!
//!
//! ## Macro Rules
//! 1. Create zero copy structs Z<StructName> for the struct
//!    1.1. The first consecutive fixed-size fields are extracted into a meta struct Z<StructName>Meta
//!    1.2. Meta extraction stops at first Vec, Option, or non-Copy type
//!    1.3. Primitive types are converted to little-endian equivalents (u16→U16, u32→U32, u64→U64, bool→u8)
//!    1.4. Fields after meta are included directly in the Z-struct and deserialized sequentially
//!    1.5. Vec<u8> uses optimized slice operations, other Vec<T> types use ZeroCopySlice
//!    1.6. Option<u64/u32/u16> are optimized, other Option<T> delegate to T's ZeroCopyAt
//!    1.7. Non-Copy types must implement ZeroCopyAt trait
//!    Examples:
//!     ```rust
//!     use light_zero_copy::slice::ZeroCopySliceBorsh;
//!     use light_zero_copy::slice_mut::ZeroCopySliceMutBorsh;
//!
//!    pub struct Struct1 {
//!        a: Vec<u8>,
//!    }
//!
//!    pub struct ZStruct1<'a> {
//!        a: &'a [u8]
//!    }
//!    pub struct ZStruct1Mut<'a> {
//!         a: &'a mut [u8]
//!    }
//!
//!     pub struct Struct2 {
//!        a: Vec<u64>,
//!    }
//!
//!     pub struct ZStruct2<'a> {
//!         a: ZeroCopySliceBorsh<'a, u64>,
//!     }
//!     pub struct ZStruct2Mut<'a> {
//!         a: ZeroCopySliceMutBorsh<'a, u64>,
//!     }
//!     ```
//! 2. Implement ZeroCopyAt trait which returns Z<StructName>
//! 3. ZeroCopyMut (separate derive) adds:
//!    3.1. Mutable variants with 'Mut' suffix (Z<StructName>Mut, Z<StructName>MetaMut)
//!    3.2. ZeroCopyAtMut trait implementation
//!    3.3. ZeroCopyNew trait with configuration struct for dynamic field initialization

use proc_macro::TokenStream;

mod shared;
mod zero_copy;
mod zero_copy_eq;
#[cfg(feature = "mut")]
mod zero_copy_mut;

#[cfg(test)]
mod tests;

/// ZeroCopy derivation macro for zero-copy deserialization
///
/// # Usage
///
/// Basic usage:
/// ```rust
/// use light_zero_copy_derive::ZeroCopy;
/// #[derive(ZeroCopy)]
/// pub struct MyStruct {
///     pub a: u8,
/// }
/// ```
///
/// To derive PartialEq as well, use ZeroCopyEq in addition to ZeroCopy:
/// ```rust
/// use light_zero_copy_derive::{ZeroCopy, ZeroCopyEq};
/// #[derive(ZeroCopy, ZeroCopyEq)]
/// pub struct MyStruct {
///       pub a: u8,
/// }
/// ```
///
#[proc_macro_derive(ZeroCopy, attributes(light_hasher, hash, skip))]
pub fn derive_zero_copy(input: TokenStream) -> TokenStream {
    let res = zero_copy::derive_zero_copy_impl(input);
    TokenStream::from(match res {
        Ok(res) => res,
        Err(err) => err.to_compile_error(),
    })
}

/// ZeroCopyEq implementation to add PartialEq for zero-copy structs.
///
/// Use this in addition to ZeroCopy when you want the generated struct to implement PartialEq:
///
/// ```rust
/// use light_zero_copy_derive::{ZeroCopy, ZeroCopyEq};
/// #[derive(ZeroCopy, ZeroCopyEq)]
/// pub struct MyStruct {
///       pub a: u8,
/// }
/// ```
/// Note: Options are not supported in ZeroCopyEq
#[proc_macro_derive(ZeroCopyEq)]
pub fn derive_zero_copy_eq(input: TokenStream) -> TokenStream {
    let res = zero_copy_eq::derive_zero_copy_eq_impl(input);
    TokenStream::from(match res {
        Ok(res) => res,
        Err(err) => err.to_compile_error(),
    })
}

/// ZeroCopyMut derivation macro for mutable zero-copy deserialization
///
/// This macro generates mutable zero-copy implementations including:
/// - ZeroCopyAtMut trait implementation
/// - Mutable Z-struct with `Mut` suffix (Z<StructName>Mut)
/// - Mutable meta struct if there are fixed-size fields (Z<StructName>MetaMut)
/// - ZeroCopyNew trait implementation with configuration support
/// - Configuration struct for dynamic fields or unit type for fixed-size structs
///
/// # Usage
///
/// ```rust
/// use light_zero_copy_derive::ZeroCopyMut;
///
/// #[derive(ZeroCopyMut)]
/// pub struct MyStruct {
///     pub a: u8,
///     pub vec: Vec<u8>,
/// }
/// ```
///
/// This will generate:
/// - `ZMyStructMut<'a>` type for mutable zero-copy access
/// - `MyStructConfig` struct with `vec: u32` field for Vec length
/// - `ZeroCopyAtMut` trait implementation for deserialization
/// - `ZeroCopyNew` trait implementation for initialization with config
///
/// For fixed-size structs, generates unit config:
/// ```rust
/// use light_zero_copy_derive::ZeroCopyMut;
/// #[derive(ZeroCopyMut)]
/// pub struct FixedStruct {
///     pub a: u8,
///     pub b: u16,
/// }
/// // Generates: pub type FixedStructConfig = ();
/// ```
///
/// For both immutable and mutable functionality, use both derives:
/// ```rust
/// use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};
///
/// #[derive(ZeroCopy, ZeroCopyMut)]
/// pub struct MyStruct {
///     pub a: u8,
/// }
/// ```
#[cfg(feature = "mut")]
#[proc_macro_derive(ZeroCopyMut)]
pub fn derive_zero_copy_mut(input: TokenStream) -> TokenStream {
    let res = zero_copy_mut::derive_zero_copy_mut_impl(input);
    TokenStream::from(match res {
        Ok(res) => res,
        Err(err) => err.to_compile_error(),
    })
}
