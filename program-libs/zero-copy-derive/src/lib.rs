use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput};

mod byte_len;
mod deserialize_impl;
mod from_impl;
mod meta_struct;
mod partial_eq_impl;
mod utils;
mod z_struct;
mod zero_copy_struct_inner;

/// ZeroCopy derivation macro for zero-copy deserialization
///
/// # Usage
///
/// Basic usage:
/// no_rust'''
/// #[derive(ZeroCopy)]
/// pub struct MyStruct {
///     pub a: u8,
/// }
/// '''
///
/// To derive PartialEq as well, use ZeroCopyEq in addition to ZeroCopy:
/// no_rust'''
/// #[derive(ZeroCopy, ZeroCopyEq)]
/// pub struct MyStruct {
///       pub a: u8,
/// }
/// '''
///
/// # Macro Rules
/// 1. Create zero copy structs Z<StructName> and Z<StructName>Mut for the struct
///     1.1. The first fields are extracted into a meta struct until we reach a Vec, Option or type that does not implement Copy
///     1.2. Represent vectors to ZeroCopySlice & don't include these into the meta struct
///     1.3. Replace u16 with U16, u32 with U32, etc
///     1.4. Every field after the first vector is directly included in the ZStruct and deserialized 1 by 1
///     1.5. If a vector contains a nested vector (does not implement Copy) it must implement Deserialize
///     1.6. Elements in an Option must implement Deserialize
///     1.7. A type that does not implement Copy must implement Deserialize, and is deserialized 1 by 1
///     1.8. is u8 deserialized as u8::zero_copy_at instead of Ref<&'a [u8], u8> for non  mut, for mut it is Ref<&'a mut [u8], u8>
/// 2. Implement Deserialize and DeserializeMut which return Z<StructName> and Z<StructName>Mut
/// 3. Implement From<Z<StructName>> for StructName and From<Z<StructName>Mut> for StructName
///
/// TODOs:
/// 1. test and fix boolean support for mut derivation (is just represented as u8)
/// 2. add more tests in particular for mut derivation
/// 3. rename deserialize traits to ZeroCopy and ZeroCopyMut
/// 4. check generated code by hand
/// 5. fix partial eq generation for options
#[proc_macro_derive(ZeroCopy)]
pub fn derive_zero_copy(input: TokenStream) -> TokenStream {
    // Parse the input DeriveInput
    let input = parse_macro_input!(input as DeriveInput);

    // // Check for both the poseidon_hasher attribute and LightHasher in derive
    // let hasher = input.attrs.iter().any(|attr| {
    //     if attr.path().is_ident("poseidon") {
    //         return true;
    //     }
    //     false
    // });
    let hasher = false;

    // Process the input to extract struct information
    let (name, z_struct_name, z_struct_meta_name, fields) = utils::process_input(&input);

    // Process the fields to separate meta fields and struct fields
    let (meta_fields, struct_fields) = utils::process_fields(fields);
    // let hasher = false;
    // Generate each implementation part using the respective modules
    // let meta_struct_def_mut =
    //     meta_struct::generate_meta_struct::<true>(&z_struct_meta_name, &meta_fields, hasher);
    let meta_struct_def =
        meta_struct::generate_meta_struct::<false>(&z_struct_meta_name, &meta_fields, hasher);

    // let z_struct_def_mut = z_struct::generate_z_struct::<true>(
    //     &z_struct_name,
    //     &z_struct_meta_name,
    //     &struct_fields,
    //     &meta_fields,
    //     hasher,
    // );
    let z_struct_def = z_struct::generate_z_struct::<false>(
        &z_struct_name,
        &z_struct_meta_name,
        &struct_fields,
        &meta_fields,
        hasher,
    );

    // let zero_copy_struct_inner_impl_mut =
    //     // For mutable version, we use the Mut suffix for the ZeroCopyInner type
    //     zero_copy_struct_inner::generate_zero_copy_struct_inner::<true>(
    //         name,
    //         &format_ident!("{}Mut", z_struct_name),
    //     );
    let zero_copy_struct_inner_impl =
        zero_copy_struct_inner::generate_zero_copy_struct_inner::<false>(name, &z_struct_name);

    let _byte_len_impl = byte_len::generate_byte_len_impl(name, &meta_fields, &struct_fields);

    // let deserialize_impl_mut = deserialize_impl::generate_deserialize_impl::<true>(
    //     name,
    //     &z_struct_name,
    //     &z_struct_meta_name,
    //     &struct_fields,
    //     meta_fields.is_empty(),
    //     quote! {},
    // );

    let deserialize_impl = deserialize_impl::generate_deserialize_impl::<false>(
        name,
        &z_struct_name,
        &z_struct_meta_name,
        &struct_fields,
        meta_fields.is_empty(),
        quote! {},
    );

    // Combine all implementations
    let expanded = quote! {
        #meta_struct_def

        // #meta_struct_def_mut

        #z_struct_def

        // #z_struct_def_mut

        #zero_copy_struct_inner_impl

        // #zero_copy_struct_inner_impl_mut

        #deserialize_impl

        // #deserialize_impl_mut

        // Don't derive byte_len for non-mut derivations
        // impl #name {
        //     #byte_len_impl
        // }

    };

    // For testing, we could add assertions here to verify the output
    TokenStream::from(expanded)
}

/// ZeroCopyEq implementation to add PartialEq for zero-copy structs.
///
/// Use this in addition to ZeroCopy when you want the generated struct to implement PartialEq:
///
/// no_rust```
/// #[derive(ZeroCopy, ZeroCopyEq)]
/// pub struct MyStruct {
///       pub a: u8,
/// }
/// ```
#[proc_macro_derive(ZeroCopyEq)]
pub fn derive_zero_copy_eq(input: TokenStream) -> TokenStream {
    // Parse the input DeriveInput
    let input = parse_macro_input!(input as DeriveInput);

    // Process the input to extract struct information
    let (name, z_struct_name, z_struct_meta_name, fields) = utils::process_input(&input);

    // Process the fields to separate meta fields and struct fields
    let (meta_fields, struct_fields) = utils::process_fields(fields);

    // Generate the PartialEq implementation.
    let partial_eq_impl = partial_eq_impl::generate_partial_eq_impl(
        name,
        &z_struct_name,
        &z_struct_meta_name,
        &meta_fields,
        &struct_fields,
    );
    // Generate From implementations
    let from_impl =
        from_impl::generate_from_impl::<false>(name, &z_struct_name, &meta_fields, &struct_fields);
    // let from_impl_mut =
    //     from_impl::generate_from_impl::<true>(name, &z_struct_name, &meta_fields, &struct_fields);

    let _z_struct_name = format_ident!("{}Mut", z_struct_name);
    let _z_struct_meta_name = format_ident!("{}Mut", z_struct_meta_name);
    // let mut_partial_eq_impl = partial_eq_impl::generate_partial_eq_impl(
    //     name,
    //     &z_struct_name,
    //     &z_struct_meta_name,
    //     &meta_fields,
    //     &struct_fields,
    // );

    TokenStream::from(quote! {
        #partial_eq_impl
        // #mut_partial_eq_impl


        #from_impl

        // #from_impl_mut
    })
}

// #[cfg(test)]
// mod tests {
//     use quote::{format_ident, quote};
//     use syn::{parse_quote, DeriveInput, Field};

//     use super::*;
//     use crate::utils::process_input;

//     // Test case setup struct for easier management of field definitions and expected results
//     struct TestCase {
//         name: &'static str,
//         fields: Vec<Field>,
//         expected_meta_fields: usize,
//         expected_struct_fields: usize,
//         assertions: Vec<(&'static str, bool)>, // pattern, should_contain
//     }

//     // Basic test for the From implementation
//     #[test]
//     fn test_from_implementation() {
//         // Create a simple struct for testing
//         let input: DeriveInput = parse_quote! {
//             #[repr(C)]
//             #[derive(Debug, PartialEq)]
//             pub struct SimpleStruct {
//                 pub a: u8,
//                 pub b: u16,
//                 pub vec: Vec<u8>,
//                 pub c: u64,
//             }
//         };

//         // Process the input to extract struct information
//         let (name, z_struct_name, _z_struct_meta_name, fields) = utils::process_input(&input);

//         // Process the fields to separate meta fields and struct fields
//         let (meta_fields, struct_fields) = utils::process_fields(fields);

//         // Generate the From implementation
//         let from_impl = from_impl::generate_from_impl::<false>(
//             name,
//             &z_struct_name,
//             &meta_fields,
//             &struct_fields,
//         );

//         // Generate the mut From implementation
//         let from_impl_mut = from_impl::generate_from_impl::<true>(
//             name,
//             &z_struct_name,
//             &meta_fields,
//             &struct_fields,
//         );

//         // Convert to string for validation
//         let from_impl_str = from_impl.to_string();
//         let from_impl_mut_str = from_impl_mut.to_string();

//         // Check that the implementations are generated correctly
//         assert!(from_impl_str.contains("impl < 'a > From < ZSimpleStruct < 'a >> for SimpleStruct"));
//         assert!(from_impl_mut_str
//             .contains("impl < 'a > From < ZSimpleStructMut < 'a >> for SimpleStruct"));

//         // Check field handling for both implementations
//         assert!(from_impl_str.contains("a :"));
//         assert!(from_impl_str.contains("b :"));
//         assert!(from_impl_str.contains("vec :"));
//         assert!(from_impl_str.contains("c :"));

//         assert!(from_impl_mut_str.contains("a :"));
//         assert!(from_impl_mut_str.contains("b :"));
//         assert!(from_impl_mut_str.contains("vec :"));
//         assert!(from_impl_mut_str.contains("c :"));
//     }

//     #[test]
//     fn test_simple_struct_generation() {
//         // Create a simple struct for testing
//         let input: DeriveInput = parse_quote! {
//             #[repr(C)]
//             #[derive(Debug, PartialEq)]
//             pub struct TestStruct {
//                 pub a: u8,
//                 pub b: u16,
//             }
//         };

//         // Process the input using our utility function
//         let (name, z_struct_name, z_struct_meta_name, fields) = process_input(&input);

//         // Run the function that processes the fields
//         let (meta_fields, struct_fields) = utils::process_fields(fields);

//         // Check that the names are correct
//         assert_eq!(name.to_string(), "TestStruct");
//         assert_eq!(z_struct_name.to_string(), "ZTestStruct");
//         assert_eq!(z_struct_meta_name.to_string(), "ZTestStructMeta");

//         // Check that fields are correctly identified
//         assert_eq!(meta_fields.len(), 2);
//         assert_eq!(struct_fields.len(), 0);

//         assert_eq!(meta_fields[0].ident.as_ref().unwrap().to_string(), "a");
//         assert_eq!(meta_fields[1].ident.as_ref().unwrap().to_string(), "b");
//     }

//     #[test]
//     fn test_compressed_account_struct() {
//         // No need to mock Pubkey, parse_quote handles it

//         // Define CompressedAccountData struct first (used within CompressedAccount)
//         let compressed_account_data_input: DeriveInput = parse_quote! {
//             #[repr(C)]
//             pub struct CompressedAccountData {
//                 pub discriminator: [u8; 8],
//                 pub data: Vec<u8>,
//                 pub data_hash: [u8; 32],
//             }
//         };

//         // Define CompressedAccount struct with the complex fields
//         let compressed_account_input: DeriveInput = parse_quote! {
//             #[repr(C)]
//             pub struct CompressedAccount {
//                 pub owner: Pubkey,
//                 pub lamports: u64,
//                 pub address: Option<[u8; 32]>,
//                 pub data: Option<CompressedAccountData>,
//             }
//         };

//         // Process CompressedAccountData first
//         let (_, _, _, fields) = process_input(&compressed_account_data_input);

//         let (meta_fields, struct_fields) = utils::process_fields(fields);

//         // Verify CompressedAccountData field splitting
//         // discriminator ([u8; 8]) is a Copy type, so it should be in meta_fields
//         assert_eq!(meta_fields.len(), 1);
//         assert_eq!(struct_fields.len(), 2); // Vec<u8> and [u8; 32] are in struct_fields

//         // Process CompressedAccount
//         let (name, z_struct_name, z_struct_meta_name, fields) =
//             process_input(&compressed_account_input);

//         let (meta_fields, struct_fields) = utils::process_fields(fields);

//         // Check struct naming
//         assert_eq!(name.to_string(), "CompressedAccount");
//         assert_eq!(z_struct_name.to_string(), "ZCompressedAccount");
//         assert_eq!(z_struct_meta_name.to_string(), "ZCompressedAccountMeta");

//         // Check field splitting
//         // Since we added Pubkey as a Copy type, owner should be in meta_fields
//         // And all other fields should be in struct_fields due to field ordering rules
//         assert_eq!(meta_fields.len(), 2);
//         assert_eq!(struct_fields.len(), 2);

//         // Check struct fields are correctly identified
//         assert_eq!(meta_fields[0].ident.as_ref().unwrap().to_string(), "owner");
//         assert_eq!(
//             meta_fields[1].ident.as_ref().unwrap().to_string(),
//             "lamports"
//         );
//         assert_eq!(
//             struct_fields[0].ident.as_ref().unwrap().to_string(),
//             "address"
//         );
//         assert_eq!(struct_fields[1].ident.as_ref().unwrap().to_string(), "data");

//         // Generate full implementation to verify - use internal functions directly instead of proc macro
//         let (name, z_struct_name, z_struct_meta_name, fields) =
//             process_input(&compressed_account_input);
//         let (meta_fields, struct_fields) = utils::process_fields(fields);

//         // Generate each implementation part using the respective modules
//         let meta_struct_def =
//             meta_struct::generate_meta_struct::<false>(&z_struct_meta_name, &meta_fields, false);
//         let z_struct_def = z_struct::generate_z_struct::<false>(
//             &z_struct_name,
//             &z_struct_meta_name,
//             &struct_fields,
//             &meta_fields,
//             false,
//         );
//         let zero_copy_struct_inner_impl =
//             zero_copy_struct_inner::generate_zero_copy_struct_inner::<false>(name, &z_struct_name);
//         let deserialize_impl = deserialize_impl::generate_deserialize_impl::<false>(
//             name,
//             &z_struct_name,
//             &z_struct_meta_name,
//             &struct_fields,
//             meta_fields.is_empty(),
//         );
//         // let partial_eq_impl = partial_eq_impl::generate_partial_eq_impl(
//         //     name,
//         //     &z_struct_name,
//         //     &z_struct_meta_name,
//         //     &meta_fields,
//         //     &struct_fields,
//         // );

//         // Combine all implementations
//         let expanded = quote! {
//             #meta_struct_def
//             #z_struct_def
//             #zero_copy_struct_inner_impl
//             #deserialize_impl
//             // #partial_eq_impl
//         };

//         let result = expanded.to_string();

//         // Create a standardized format for comparison by removing whitespace and normalizing syntax
//         fn normalize_code(code: &str) -> String {
//             code.chars()
//                 .filter(|c| !c.is_whitespace())
//                 .collect::<String>()
//         }

//         // Get the normalized actual result first
//         let normalized_result = normalize_code(&result);
//         println!("Generated code normalized:\n{}", normalized_result);
//         // Print the generated code for debugging purposes
//         println!(
//             "Generated code normalized:\n{}",
//             String::from_utf8(rustfmt(result)).unwrap()
//         );

//         // Directly verify key structural elements instead of doing a full string comparison
//         assert!(normalized_result.contains("pubstructZCompressedAccountMeta{"));
//         assert!(normalized_result.contains("pubstructZCompressedAccount<'a>"));
//         assert!(
//             normalized_result.contains("meta:light_zero_copy::Ref<&'a[u8],ZCompressedAccountMeta>")
//         );
//         assert!(normalized_result.contains("pubowner:"));
//         assert!(normalized_result.contains("publamports:"));
//         assert!(normalized_result.contains("pubaddress:<Option<[u8;32]>"));
//         assert!(normalized_result.contains("pubdata:<Option<CompressedAccountData>"));
//         assert!(normalized_result
//             .contains("impllight_zero_copy::borsh::DeserializeforCompressedAccount"));
//         assert!(normalized_result.contains("typeZeroCopyInner=ZCompressedAccount<'static>"));
//     }

//     use std::{
//         env,
//         io::{self, prelude::*},
//         process::{Command, Stdio},
//         thread::spawn,
//     };
//     pub fn rustfmt(code: String) -> Vec<u8> {
//         let mut cmd = match env::var_os("RUSTFMT") {
//             Some(r) => Command::new(r),
//             _ => Command::new("rustfmt"),
//         };

//         let mut cmd = cmd
//             .stdin(Stdio::piped())
//             .stdout(Stdio::piped())
//             .stderr(Stdio::piped())
//             .spawn()
//             .unwrap();

//         let mut stdin = cmd.stdin.take().unwrap();
//         let mut stdout = cmd.stdout.take().unwrap();

//         let stdin_handle = spawn(move || {
//             stdin.write_all(code.as_bytes()).unwrap();
//         });

//         let mut formatted_code = vec![];
//         io::copy(&mut stdout, &mut formatted_code).unwrap();

//         let _ = cmd.wait();
//         stdin_handle.join().unwrap();
//         formatted_code
//     }
//     #[test]
//     fn test_empty_struct() {
//         // Create an empty struct for testing
//         let input: DeriveInput = parse_quote! {
//             #[repr(C)]
//             pub struct EmptyStruct {}
//         };

//         // Process the input
//         let (name, z_struct_name, z_struct_meta_name, fields) = process_input(&input);

//         // Split into meta fields and struct fields
//         let (meta_fields, struct_fields) = utils::process_fields(fields);

//         // Generate each implementation part
//         let meta_struct_def =
//             meta_struct::generate_meta_struct::<false>(&z_struct_meta_name, &meta_fields, false);
//         let z_struct_def = z_struct::generate_z_struct::<false>(
//             &z_struct_name,
//             &z_struct_meta_name,
//             &struct_fields,
//             &meta_fields,
//             false,
//         );
//         let zero_copy_struct_inner_impl =
//             zero_copy_struct_inner::generate_zero_copy_struct_inner::<false>(name, &z_struct_name);
//         let deserialize_impl = deserialize_impl::generate_deserialize_impl::<false>(
//             name,
//             &z_struct_name,
//             &z_struct_meta_name,
//             &struct_fields,
//             meta_fields.is_empty(),
//         );
//         let partial_eq_impl = partial_eq_impl::generate_partial_eq_impl(
//             name,
//             &z_struct_name,
//             &z_struct_meta_name,
//             &meta_fields,
//             &struct_fields,
//         );

//         // Combine all implementations
//         let expanded = quote! {
//             #meta_struct_def
//             #z_struct_def
//             #zero_copy_struct_inner_impl
//             #deserialize_impl
//             #partial_eq_impl
//         };

//         // Convert to string for validation
//         let result = expanded.to_string();

//         // Verify the output contains what we expect
//         assert!(result.contains("struct ZEmptyStructMeta"));
//         assert!(result.contains("struct ZEmptyStruct < 'a >"));
//         assert!(
//             result.contains("impl light_zero_copy :: borsh :: ZeroCopyStructInner for EmptyStruct")
//         );
//         assert!(result.contains(
//             "impl < 'a > light_zero_copy :: borsh :: Deserialize < 'a > for EmptyStruct"
//         ));
//     }

//     #[test]
//     fn test_struct_with_bool() {
//         // Create a struct with bool fields for testing
//         let input: DeriveInput = parse_quote! {
//             #[repr(C)]
//             pub struct BoolStruct {
//                 pub a: bool,
//                 pub b: u8,
//                 pub c: Vec<u8>,
//                 pub d: bool,
//             }
//         };

//         // Process the input
//         let (_, z_struct_name, z_struct_meta_name, fields) = process_input(&input);

//         // Split into meta fields and struct fields
//         let (meta_fields, struct_fields) = utils::process_fields(fields);

//         // Check that fields are correctly identified
//         assert_eq!(meta_fields.len(), 2); // 'a' and 'b' should be in meta_fields
//         assert_eq!(struct_fields.len(), 2); // 'c' and 'd' should be in struct_fields

//         // Generate the implementation
//         let meta_struct_def =
//             meta_struct::generate_meta_struct::<false>(&z_struct_meta_name, &meta_fields, false);
//         let z_struct_def = z_struct::generate_z_struct::<false>(
//             &z_struct_name,
//             &z_struct_meta_name,
//             &struct_fields,
//             &meta_fields,
//             false,
//         );

//         // Check meta struct has bool converted to u8
//         let meta_struct_str = meta_struct_def.to_string();
//         println!("meta_struct_str {}", meta_struct_str);
//         assert!(meta_struct_str.contains("pub a : u8"));

//         // Check z_struct has methods for boolean fields
//         let z_struct_str = z_struct_def.to_string();
//         println!("z_struct_str {}", z_struct_str);
//         assert!(z_struct_str.contains("pub fn a (& self) -> bool {"));
//         assert!(z_struct_str.contains("self . a > 0"));
//         assert!(z_struct_str.contains("pub fn d (& self) -> bool {"));
//         assert!(z_struct_str.contains("self . d > 0"));
//     }

//     #[test]
//     fn test_zero_copy_eq() {
//         // Create a test input
//         let input: DeriveInput = parse_quote! {
//             #[repr(C)]
//             pub struct TestStruct {
//                 pub a: u8,
//                 pub b: u16,
//             }
//         };

//         // Process the input for ZeroCopy
//         let (name, z_struct_name, z_struct_meta_name, fields) = process_input(&input);
//         let (meta_fields, struct_fields) = utils::process_fields(fields);

//         // Generate code from ZeroCopy
//         let meta_struct_def =
//             meta_struct::generate_meta_struct::<false>(&z_struct_meta_name, &meta_fields, false);
//         let z_struct_def = z_struct::generate_z_struct::<false>(
//             &z_struct_name,
//             &z_struct_meta_name,
//             &struct_fields,
//             &meta_fields,
//             false,
//         );
//         let zero_copy_struct_inner_impl =
//             zero_copy_struct_inner::generate_zero_copy_struct_inner::<false>(name, &z_struct_name);
//         let deserialize_impl = deserialize_impl::generate_deserialize_impl::<false>(
//             name,
//             &z_struct_name,
//             &z_struct_meta_name,
//             &struct_fields,
//             meta_fields.is_empty(),
//         );

//         let zero_copy_expanded = quote! {
//             #meta_struct_def
//             #z_struct_def
//             #zero_copy_struct_inner_impl
//             #deserialize_impl
//         };

//         // Generate code from ZeroCopyEq
//         let partial_eq_impl = partial_eq_impl::generate_partial_eq_impl(
//             name,
//             &z_struct_name,
//             &z_struct_meta_name,
//             &meta_fields,
//             &struct_fields,
//         );

//         // Verify ZeroCopy output doesn't include PartialEq
//         let zero_copy_result = zero_copy_expanded.to_string();
//         assert!(
//             !zero_copy_result.contains("impl < 'a > PartialEq < TestStruct >"),
//             "ZeroCopy alone should not include PartialEq implementation"
//         );

//         // Verify ZeroCopyEq output is just the PartialEq implementation
//         let zero_copy_eq_result = partial_eq_impl.to_string();
//         assert!(
//             zero_copy_eq_result.contains("impl < 'a > PartialEq < TestStruct >"),
//             "ZeroCopyEq should include PartialEq implementation"
//         );

//         // Verify that combining both gives us the complete implementation
//         let combined = quote! {
//             #zero_copy_expanded

//             #partial_eq_impl
//         };

//         let combined_result = combined.to_string();
//         assert!(
//             combined_result.contains("impl < 'a > PartialEq < TestStruct >"),
//             "Combining ZeroCopy and ZeroCopyEq should include PartialEq implementation"
//         );
//     }

//     #[test]
//     fn test_struct_with_vector() {
//         // Create a struct with Vec<u8> field for testing
//         let input: DeriveInput = parse_quote! {
//             #[repr(C)]
//             pub struct VecStruct {
//                 pub a: u8,
//                 pub b: Vec<u8>,
//                 pub c: u32,
//             }
//         };

//         // Process the input
//         let (_name, _z_struct_name, z_struct_meta_name, fields) = process_input(&input);

//         // Split into meta fields and struct fields
//         let (meta_fields, struct_fields) = utils::process_fields(fields);

//         // Check that fields are correctly identified
//         assert_eq!(meta_fields.len(), 1); // Only 'a' should be in meta_fields
//         assert_eq!(struct_fields.len(), 2); // 'b' and 'c' should be in struct_fields

//         // The field names should be correct
//         assert_eq!(meta_fields[0].ident.as_ref().unwrap().to_string(), "a");
//         assert_eq!(struct_fields[0].ident.as_ref().unwrap().to_string(), "b");
//         assert_eq!(struct_fields[1].ident.as_ref().unwrap().to_string(), "c");

//         // Generate the implementation
//         let meta_struct_def =
//             meta_struct::generate_meta_struct::<false>(&z_struct_meta_name, &meta_fields, false);
//         let result = meta_struct_def.to_string();

//         // Verify the meta struct has the right fields
//         assert!(result.contains("pub a : u8"));
//         assert!(!result.contains("pub b : Vec < u8 >"));
//     }

//     #[test]
//     fn test_mutable_attribute() {
//         // Create a simple struct with the mutable attribute
//         let input: DeriveInput = parse_quote! {
//             #[derive(ZeroCopy)]
//             #[zero_copy(mutable)]
//             pub struct MutableStruct {
//                 pub a: u8,
//                 pub b: Vec<u8>,
//             }
//         };

//         // Check for the mutable attribute
//         let mut is_mutable = false;
//         for attr in &input.attrs {
//             if attr.path().is_ident("zero_copy") {
//                 let _ = attr.parse_nested_meta(|meta| {
//                     if meta.path.is_ident("mutable") {
//                         is_mutable = true;
//                     }
//                     Ok(())
//                 });
//             }
//         }

//         // Verify the mutable attribute is detected
//         assert!(is_mutable, "Mutable attribute should be detected");

//         // Process the input
//         let (name, z_struct_name, z_struct_meta_name, fields) = process_input(&input);
//         let (meta_fields, struct_fields) = utils::process_fields(fields);

//         // Generate the expanded code
//         let meta_struct_def =
//             meta_struct::generate_meta_struct::<true>(&z_struct_meta_name, &meta_fields, false);
//         let z_struct_def = z_struct::generate_z_struct::<true>(
//             &z_struct_name,
//             &z_struct_meta_name,
//             &struct_fields,
//             &meta_fields,
//             false,
//         );
//         let zero_copy_struct_inner_impl = zero_copy_struct_inner::generate_zero_copy_struct_inner::<
//             false,
//         >(name, &format_ident!("{}Mut", z_struct_name));
//         let deserialize_impl = deserialize_impl::generate_deserialize_impl::<true>(
//             name,
//             &z_struct_name,
//             &z_struct_meta_name,
//             &struct_fields,
//             meta_fields.is_empty(),
//         );

//         // Combine all implementations
//         let expanded = quote! {
//             #meta_struct_def
//             #z_struct_def
//             #zero_copy_struct_inner_impl
//             #deserialize_impl
//         };

//         let result = expanded.to_string();

//         // Verify mutable-specific code generation
//         println!("Generated code: {}", result);
//         assert!(
//             result.contains("ZMutableStructMut"),
//             "Mutable implementation should add Mut suffix to type name"
//         );
//         assert!(
//             result.contains("light_zero_copy :: borsh_mut ::"),
//             "Mutable implementation should use borsh_mut"
//         );
//         assert!(
//             result.contains("& 'a mut [u8]"),
//             "Mutable implementation should use & 'a mut [u8]"
//         );
//         assert!(
//             result.contains("borsh_vec_u8_as_slice_mut"),
//             "Mutable implementation should use borsh_vec_u8_as_slice_mut"
//         );
//     }

//     #[test]
//     fn test_derive_zero_copy_edge_cases() {
//         // Define test cases covering edge cases based on the rules
//         let test_cases = vec![
//             // Case 1: Empty struct
//             TestCase {
//                 name: "EmptyStruct",
//                 fields: vec![],
//                 expected_meta_fields: 0,
//                 expected_struct_fields: 0,
//                 assertions: vec![
//                     ("struct ZEmptyStructMeta { }", true),
//                     ("impl light_zero_copy :: borsh :: Deserialize for EmptyStruct", true),
//                 ],
//             },

//             // Case 2: All primitive Copy types
//             TestCase {
//                 name: "AllPrimitives",
//                 fields: vec![
//                     parse_quote!(pub a: u8),
//                     parse_quote!(pub b: u16),
//                     parse_quote!(pub c: u32),
//                     parse_quote!(pub d: u64),
//                     parse_quote!(pub e: bool),
//                 ],
//                 expected_meta_fields: 5,
//                 expected_struct_fields: 0,
//                 assertions: vec![
//                     ("pub a : u8", true),
//                     ("pub b : light_zero_copy :: little_endian :: U16", true), // Rule 1.3: Replace u16 with U16
//                     ("pub c : light_zero_copy :: little_endian :: U32", true), // Rule 1.3: Replace u32 with U32
//                     ("pub d : light_zero_copy :: little_endian :: U64", true), // Rule 1.3: Replace u64 with U64
//                     ("pub e : u8", true),
//                     ("meta : light_zero_copy :: Ref < & 'a [u8] , ZAllPrimitivesMeta >", true),
//                 ],
//             },

//             // Case 3: Vec<u8> at start (Rule 1.1)
//             TestCase {
//                 name: "VecAtStart",
//                 fields: vec![
//                     parse_quote!(pub data: Vec<u8>),
//                     parse_quote!(pub a: u8),
//                     parse_quote!(pub b: u16),
//                 ],
//                 expected_meta_fields: 0,
//                 expected_struct_fields: 3,
//                 assertions: vec![
//                     ("pub data : & 'a [u8]", true), // Rule 1.2: Vec<u8> represented as slice
//                     ("struct ZVecAtStartMeta { }", true), // Empty meta struct
//                     ("let (data , bytes) = light_zero_copy :: borsh :: borsh_vec_u8_as_slice (bytes) ?", true),
//                 ],
//             },

//             // Case 4: Vec in middle (Rule 1.1, 1.4)
//             TestCase {
//                 name: "VecInMiddle",
//                 fields: vec![
//                     parse_quote!(pub a: u8),
//                     parse_quote!(pub b: u16),
//                     parse_quote!(pub data: Vec<u8>), // Split point
//                     parse_quote!(pub c: u32),
//                     parse_quote!(pub d: u64),
//                 ],
//                 expected_meta_fields: 2,
//                 expected_struct_fields: 3,
//                 assertions: vec![
//                     ("struct ZVecInMiddleMeta { pub a : u8 , pub b : light_zero_copy :: little_endian :: U16 , }", true),
//                     ("pub data : & 'a [u8]", true),
//                     ("let (c , bytes) = light_zero_copy :: Ref :: < & 'a [u8] , light_zero_copy :: little_endian :: U32 > :: from_prefix (bytes) ?", true),
//                     ("let (d , bytes) = light_zero_copy :: Ref :: < & 'a [u8] , light_zero_copy :: little_endian :: U64 > :: from_prefix (bytes) ?", true),
//                 ],
//             },

//             // Case 5: Mixed Vec<T> types (Rules 1.5)
//             TestCase {
//                 name: "MixedVecTypes",
//                 fields: vec![
//                     parse_quote!(pub a: u8),
//                     parse_quote!(pub bytes: Vec<u8>), // Vec<u8> special case
//                     parse_quote!(pub numbers: Vec<u32>), // Vec with Copy type
//                 ],
//                 expected_meta_fields: 1,
//                 expected_struct_fields: 2,
//                 assertions: vec![
//                     ("pub bytes : & 'a [u8]", true), // Rule 1.2: Vec<u8> as slice
//                     ("pub numbers : light_zero_copy :: slice :: ZeroCopySliceBorsh < 'a ,", true), // Using ZeroCopySliceBorsh for Copy types
//                     ("let (bytes , bytes) = light_zero_copy :: borsh :: borsh_vec_u8_as_slice (bytes) ?", true),
//                     ("let (numbers , bytes) = light_zero_copy :: slice :: ZeroCopySliceBorsh", true),
//                 ],
//             },

//             // Case 6: Option type splitting boundary (Rule 1.6)
//             TestCase {
//                 name: "OptionTypeStruct",
//                 fields: vec![
//                     parse_quote!(pub a: u8),
//                     parse_quote!(pub b: Option<u32>), // Split point
//                     parse_quote!(pub c: u64),
//                 ],
//                 expected_meta_fields: 1,
//                 expected_struct_fields: 2,
//                 assertions: vec![
//                     ("struct ZOptionTypeStructMeta { pub a : u8 , }", true),
//                     ("pub b : < Option < u32 > as light_zero_copy :: borsh :: Deserialize> :: Output< 'a >", true),
//                     ("let (b , bytes) = < Option < u32 > as light_zero_copy :: borsh :: Deserialize > :: zero_copy_at (bytes) ?", true),
//                 ],
//             },

//             // Case 7: Arrays should be treated as Copy types
//             TestCase {
//                 name: "ArrayTypes",
//                 fields: vec![
//                     parse_quote!(pub a: [u8; 4]),
//                     parse_quote!(pub b: [u32; 2]),
//                     parse_quote!(pub c: Vec<u8>), // Split point
//                 ],
//                 expected_meta_fields: 2,
//                 expected_struct_fields: 1,
//                 assertions: vec![
//                     // Just check for the existence of the array field types, not exact formatting
//                     ("pub a : [u8 ; 4]", true),
//                     ("pub b : [u32 ; 2]", true), // Arrays don't use zerocopy types
//                     ("pub c : & 'a [u8]", true),
//                 ],
//             },

//             // Case 8: Test field after Option (Rule 1.4)
//             TestCase {
//                 name: "FieldsAfterNonCopy",
//                 fields: vec![
//                     parse_quote!(pub a: u8),
//                     parse_quote!(pub opt: Option<u16>), // Split point
//                     parse_quote!(pub b: u16), // After non-Copy, should be in struct_fields
//                     parse_quote!(pub c: u32),
//                 ],
//                 expected_meta_fields: 1,
//                 expected_struct_fields: 3,
//                 assertions: vec![
//                     ("let (opt , bytes) = < Option < u16 > as light_zero_copy :: borsh :: Deserialize > :: zero_copy_at (bytes) ?", true),
//                     ("let (b , bytes) = light_zero_copy :: Ref :: < & 'a [u8] , light_zero_copy :: little_endian :: U16 > :: from_prefix (bytes) ?", true),
//                     ("let (c , bytes) = light_zero_copy :: Ref :: < & 'a [u8] , light_zero_copy :: little_endian :: U32 > :: from_prefix (bytes) ?", true),
//                 ],
//             },
//         ];

//         // Run all test cases
//         for (i, test_case) in test_cases.iter().enumerate() {
//             println!("Testing case {}: {}", i, test_case.name);

//             // Create struct
//             let struct_name = format_ident!("{}", test_case.name);
//             let mut fields_punctuated =
//                 syn::punctuated::Punctuated::<syn::Field, syn::token::Comma>::new();
//             for field in &test_case.fields {
//                 fields_punctuated.push(field.clone());
//             }

//             let input = parse_quote! {
//                 #[repr(C)]
//                 pub struct #struct_name {
//                     #fields_punctuated
//                 }
//             };

//             // Process input
//             let (name, z_struct_name, z_struct_meta_name, fields) = process_input(&input);
//             let (meta_fields, struct_fields) = utils::process_fields(fields);

//             // Verify field counts
//             assert_eq!(
//                 meta_fields.len(),
//                 test_case.expected_meta_fields,
//                 "Case {}: Expected {} meta fields, got {}",
//                 i,
//                 test_case.expected_meta_fields,
//                 meta_fields.len()
//             );
//             assert_eq!(
//                 struct_fields.len(),
//                 test_case.expected_struct_fields,
//                 "Case {}: Expected {} struct fields, got {}",
//                 i,
//                 test_case.expected_struct_fields,
//                 struct_fields.len()
//             );

//             // Generate code
//             let meta_struct_def = meta_struct::generate_meta_struct::<false>(
//                 &z_struct_meta_name,
//                 &meta_fields,
//                 false,
//             );
//             let z_struct_def = z_struct::generate_z_struct::<false>(
//                 &z_struct_name,
//                 &z_struct_meta_name,
//                 &struct_fields,
//                 &meta_fields,
//                 false,
//             );
//             let zero_copy_struct_inner_impl =
//                 zero_copy_struct_inner::generate_zero_copy_struct_inner::<false>(
//                     name,
//                     &z_struct_name,
//                 );
//             let deserialize_impl = deserialize_impl::generate_deserialize_impl::<false>(
//                 name,
//                 &z_struct_name,
//                 &z_struct_meta_name,
//                 &struct_fields,
//                 meta_fields.is_empty(),
//             );
//             let partial_eq_impl = if test_case.name != "OptionTypeStruct" {
//                 partial_eq_impl::generate_partial_eq_impl(
//                     name,
//                     &z_struct_name,
//                     &z_struct_meta_name,
//                     &meta_fields,
//                     &struct_fields,
//                 )
//             } else {
//                 quote! {}
//             };

//             // Combine all implementations
//             let expanded = quote! {
//                 #meta_struct_def
//                 #z_struct_def
//                 #zero_copy_struct_inner_impl
//                 #deserialize_impl
//                 #partial_eq_impl
//             };

//             // Convert to string for validation
//             let result = expanded.to_string();

//             // For debugging in case of a failure
//             if false {
//                 // Only enable when debugging
//                 println!("Generated code sample for case {}: {:.500}...", i, result);
//             }

//             // Verify assertions
//             for (pattern, should_contain) in &test_case.assertions {
//                 let contains = result.contains(pattern);
//                 assert_eq!(
//                     contains,
//                     *should_contain,
//                     "Case {}: Expected '{}' to be {} in the generated code",
//                     i,
//                     pattern,
//                     if *should_contain { "present" } else { "absent" }
//                 );
//             }
//         }
//     }
// }
