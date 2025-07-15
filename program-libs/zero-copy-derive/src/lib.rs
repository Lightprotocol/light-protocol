//! Procedural macros for zero-copy deserialization.
//!
//! This crate provides derive macros that generate efficient zero-copy data structures
//! and deserialization code, eliminating the need for data copying during parsing.
//!
//! ## Main Macros
//!
//! - `ZeroCopy`: Generates zero-copy structs and deserialization traits
//! - `ZeroCopyMut`: Adds mutable zero-copy support
//! - `ZeroCopyEq`: Adds PartialEq implementation for comparing with original structs
//! - `ZeroCopyNew`: Generates configuration structs for initialization

use proc_macro::TokenStream;

mod shared;
mod zero_copy;
mod zero_copy_eq;
#[cfg(feature = "mut")]
mod zero_copy_mut;

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
/// # Macro Rules
/// 1. Create zero copy structs Z<StructName> and Z<StructName>Mut for the struct
///    1.1. The first fields are extracted into a meta struct until we reach a Vec, Option or type that does not implement Copy
///    1.2. Represent vectors to ZeroCopySlice & don't include these into the meta struct
///    1.3. Replace u16 with U16, u32 with U32, etc
///    1.4. Every field after the first vector is directly included in the ZStruct and deserialized 1 by 1
///    1.5. If a vector contains a nested vector (does not implement Copy) it must implement Deserialize
///    1.6. Elements in an Option must implement Deserialize
///    1.7. A type that does not implement Copy must implement Deserialize, and is deserialized 1 by 1
///    1.8. is u8 deserialized as u8::zero_copy_at instead of Ref<&'a [u8], u8> for non  mut, for mut it is Ref<&'a mut [u8], u8>
/// 2. Implement Deserialize and DeserializeMut which return Z<StructName> and Z<StructName>Mut
/// 3. Implement From<Z<StructName>> for StructName and From<Z<StructName>Mut> for StructName
///
/// Note: Options are not supported in ZeroCopyEq
#[proc_macro_derive(ZeroCopy)]
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
/// - DeserializeMut trait implementation
/// - Mutable Z-struct with `Mut` suffix
/// - byte_len() method implementation
/// - Mutable ZeroCopyStructInner implementation
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
/// - `MyStruct::zero_copy_at_mut()` method
/// - `ZMyStructMut<'a>` type for mutable zero-copy access
/// - `MyStruct::byte_len()` method
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

// /// ZeroCopyNew derivation macro for configuration-based zero-copy initialization
// ///
// /// This macro generates configuration structs and initialization methods for structs
// /// with Vec and Option fields that need to be initialized with specific configurations.
// ///
// /// # Usage
// ///
// /// ```ignore
// /// use light_zero_copy_derive::ZeroCopyNew;
// ///
// /// #[derive(ZeroCopyNew)]
// /// pub struct MyStruct {
// ///     pub a: u8,
// ///     pub vec: Vec<u8>,
// ///     pub option: Option<u64>,
// /// }
// /// ```
// ///
// /// This will generate:
// /// - `MyStructConfig` struct with configuration fields
// /// - `ZeroCopyNew` implementation for `MyStruct`
// /// - `new_zero_copy(bytes, config)` method for initialization
// ///
// /// The configuration struct will have fields based on the complexity of the original fields:
// /// - `Vec<Primitive>` → `field_name: u32` (length)
// /// - `Option<Primitive>` → `field_name: bool` (is_some)
// /// - `Vec<Complex>` → `field_name: Vec<ComplexConfig>` (config per element)
// /// - `Option<Complex>` → `field_name: Option<ComplexConfig>` (config if some)
// #[cfg(feature = "mut")]
// #[proc_macro_derive(ZeroCopyNew)]
// pub fn derive_zero_copy_config(input: TokenStream) -> TokenStream {
//     let res = zero_copy_new::derive_zero_copy_config_impl(input);
//     TokenStream::from(match res {
//         Ok(res) => res,
//         Err(err) => err.to_compile_error(),
//     })
// }
