use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Attribute, Data, DeriveInput, Field, Fields, FieldsNamed, Ident, Type, TypePath};

// Global cache for storing whether a struct implements Copy
lazy_static::lazy_static! {
    static ref COPY_IMPL_CACHE: Arc<Mutex<HashMap<String, bool>>> = Arc::new(Mutex::new(HashMap::new()));
}

/// Creates a unique cache key for a type using span information to avoid collisions
/// between types with the same name from different modules/locations
fn create_unique_type_key(ident: &Ident) -> String {
    format!("{}:{:?}", ident, ident.span())
}

/// Process the derive input to extract the struct information
pub fn process_input(
    input: &DeriveInput,
) -> syn::Result<(
    &Ident,             // Original struct name
    proc_macro2::Ident, // Z-struct name
    proc_macro2::Ident, // Z-struct meta name
    &FieldsNamed,       // Struct fields
)> {
    let name = &input.ident;
    let z_struct_name = format_ident!("Z{}", name);
    let z_struct_meta_name = format_ident!("Z{}Meta", name);

    // Populate the cache by checking if this struct implements Copy
    let _ = struct_implements_copy(input);

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields,
            _ => {
                return Err(syn::Error::new_spanned(
                    &data.fields,
                    "ZeroCopy only supports structs with named fields",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "ZeroCopy only supports structs",
            ))
        }
    };

    Ok((name, z_struct_name, z_struct_meta_name, fields))
}

pub fn process_fields(fields: &FieldsNamed) -> (Vec<&Field>, Vec<&Field>) {
    let mut meta_fields = Vec::new();
    let mut struct_fields = Vec::new();
    let mut reached_vec_or_option = false;

    for field in fields.named.iter() {
        if !reached_vec_or_option {
            if is_vec_or_option(&field.ty) || !is_copy_type(&field.ty) {
                reached_vec_or_option = true;
                struct_fields.push(field);
            } else {
                meta_fields.push(field);
            }
        } else {
            struct_fields.push(field);
        }
    }

    (meta_fields, struct_fields)
}

pub fn is_vec_or_option(ty: &Type) -> bool {
    is_vec_type(ty) || is_option_type(ty)
}

pub fn is_vec_type(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(segment) = path.segments.last() {
            return segment.ident == "Vec";
        }
    }
    false
}

pub fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(segment) = path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

pub fn get_vec_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(segment) = path.segments.last() {
            if segment.ident == "Vec" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
    }
    None
}

pub fn get_option_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(segment) = path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
    }
    None
}

pub fn is_primitive_integer(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(segment) = path.segments.last() {
            let ident = &segment.ident;
            return ident == "u16"
                || ident == "u32"
                || ident == "u64"
                || ident == "i16"
                || ident == "i32"
                || ident == "i64"
                || ident == "u8"
                || ident == "i8";
        }
    }
    false
}

pub fn is_bool_type(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(segment) = path.segments.last() {
            return segment.ident == "bool";
        }
    }
    false
}

/// Check if a type is a specific primitive type (u8, u16, u32, u64, bool, etc.)
pub fn is_specific_primitive_type(ty: &Type, type_name: &str) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(segment) = path.segments.last() {
            return segment.ident == type_name;
        }
    }
    false
}

pub fn is_pubkey_type(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(segment) = path.segments.last() {
            return segment.ident == "Pubkey";
        }
    }
    false
}

pub fn convert_to_zerocopy_type(ty: &Type) -> TokenStream {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            if let Some(segment) = path.segments.last() {
                let ident = &segment.ident;

                // Handle primitive types first
                match ident.to_string().as_str() {
                    "u16" => quote! { light_zero_copy::little_endian::U16 },
                    "u32" => quote! { light_zero_copy::little_endian::U32 },
                    "u64" => quote! { light_zero_copy::little_endian::U64 },
                    "bool" => quote! { u8 },
                    _ => {
                        // Handle container types recursively
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            let transformed_args: Vec<TokenStream> = args
                                .args
                                .iter()
                                .map(|arg| {
                                    if let syn::GenericArgument::Type(inner_type) = arg {
                                        convert_to_zerocopy_type(inner_type)
                                    } else {
                                        quote! { #arg }
                                    }
                                })
                                .collect();

                            quote! { #ident<#(#transformed_args),*> }
                        } else {
                            quote! { #ty }
                        }
                    }
                }
            } else {
                quote! { #ty }
            }
        }
        _ => {
            quote! { #ty }
        }
    }
}

/// Checks if a struct has a derive(Copy) attribute
fn struct_has_copy_derive(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("derive") && {
            let mut found_copy = false;
            // Use parse_nested_meta as the primary and only approach - it's the syn 2.0 standard
            // for parsing comma-separated derive items like #[derive(Copy, Clone, Debug)]
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("Copy") {
                    found_copy = true;
                }
                Ok(()) // Continue parsing other derive items
            })
            .is_ok()
                && found_copy
        }
    })
}

/// Determines whether a struct implements Copy by checking for the #[derive(Copy)] attribute.
/// Results are cached for performance.
///
/// In Rust, a struct can only implement Copy if:
/// 1. It explicitly has a #[derive(Copy)] attribute, AND
/// 2. All of its fields implement Copy
///
/// The Rust compiler will enforce the second condition at compile time, so we only need to check
/// for the derive attribute here.
pub fn struct_implements_copy(input: &DeriveInput) -> bool {
    let cache_key = create_unique_type_key(&input.ident);

    // Check the cache first
    if let Some(implements_copy) = COPY_IMPL_CACHE.lock().unwrap().get(&cache_key) {
        return *implements_copy;
    }

    // Check if the struct has a derive(Copy) attribute
    let implements_copy = struct_has_copy_derive(&input.attrs);

    // Cache the result
    COPY_IMPL_CACHE
        .lock()
        .unwrap()
        .insert(cache_key, implements_copy);

    implements_copy
}

/// Determines whether a type implements Copy
/// 1. check whether type is a primitive type that implements Copy
/// 2. check whether type is an array type (which is always Copy if the element type is Copy)
/// 3. check whether type is struct -> check in the COPY_IMPL_CACHE if we know whether it has a #[derive(Copy)] attribute
///
/// For struct types, this relies on the cache populated by struct_implements_copy. If we don't have cached
/// information, it assumes the type does not implement Copy. This is a limitation of our approach, but it
/// works well in practice because process_input will call struct_implements_copy for all structs before
/// they might be referenced by other structs.
pub fn is_copy_type(ty: &Type) -> bool {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            if let Some(segment) = path.segments.last() {
                let ident = &segment.ident;

                // Check if it's a primitive type that implements Copy
                if ident == "u8"
                    || ident == "u16"
                    || ident == "u32"
                    || ident == "u64"
                    || ident == "i8"
                    || ident == "i16"
                    || ident == "i32"
                    || ident == "i64"
                    || ident == "bool" // bool is a Copy type
                    || ident == "char"
                    || ident == "Pubkey"
                // Pubkey is hardcoded as copy type for now.
                {
                    return true;
                }

                // Check if we have cached information about this type
                let cache_key = create_unique_type_key(ident);
                if let Some(implements_copy) = COPY_IMPL_CACHE.lock().unwrap().get(&cache_key) {
                    return *implements_copy;
                }
            }
        }
        // Handle array types (which are always Copy if the element type is Copy)
        Type::Array(array) => {
            // Arrays are Copy if their element type is Copy
            return is_copy_type(&array.elem);
        }
        // For struct types not in cache, we'd need the derive input to check attributes
        _ => {}
    }
    false
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    // Helper function to check if a struct implements Copy
    fn check_struct_implements_copy(input: syn::DeriveInput) -> bool {
        struct_implements_copy(&input)
    }

    #[test]
    fn test_struct_implements_copy() {
        // Ensure the cache is cleared and the lock is released immediately
        COPY_IMPL_CACHE.lock().unwrap().clear();
        // Test case 1: Empty struct with #[derive(Copy)]
        let input: syn::DeriveInput = parse_quote! {
            #[derive(Copy, Clone)]
            struct EmptyStruct {}
        };
        assert!(
            check_struct_implements_copy(input),
            "EmptyStruct should implement Copy with #[derive(Copy)]"
        );

        // Test case 2: Simple struct with #[derive(Copy)]
        let input: syn::DeriveInput = parse_quote! {
            #[derive(Copy, Clone)]
            struct SimpleStruct {
                a: u8,
                b: u16,
            }
        };
        assert!(
            check_struct_implements_copy(input),
            "SimpleStruct should implement Copy with #[derive(Copy)]"
        );

        // Test case 3: Struct with #[derive(Clone)] but not Copy
        let input: syn::DeriveInput = parse_quote! {
            #[derive(Clone)]
            struct StructWithoutCopy {
                a: u8,
                b: u16,
            }
        };
        assert!(
            !check_struct_implements_copy(input),
            "StructWithoutCopy should not implement Copy without #[derive(Copy)]"
        );

        // Test case 4: Struct with a non-Copy field but with derive(Copy)
        // Note: In real Rust code, this would not compile, but for our test we only check attributes
        let input: syn::DeriveInput = parse_quote! {
            #[derive(Copy, Clone)]
            struct StructWithVec {
                a: u8,
                b: Vec<u8>,
            }
        };
        assert!(
            check_struct_implements_copy(input),
            "StructWithVec has #[derive(Copy)] so our function returns true"
        );

        // Test case 5: Struct with all Copy fields but without #[derive(Copy)]
        let input: syn::DeriveInput = parse_quote! {
            struct StructWithCopyFields {
                a: u8,
                b: u16,
                c: i32,
                d: bool,
            }
        };
        assert!(
            !check_struct_implements_copy(input),
            "StructWithCopyFields should not implement Copy without #[derive(Copy)]"
        );

        // Test case 6: Unit struct without #[derive(Copy)]
        let input: syn::DeriveInput = parse_quote! {
            struct UnitStructWithoutCopy;
        };
        assert!(
            !check_struct_implements_copy(input),
            "UnitStructWithoutCopy should not implement Copy without #[derive(Copy)]"
        );

        // Test case 7: Unit struct with #[derive(Copy)]
        let input: syn::DeriveInput = parse_quote! {
            #[derive(Copy, Clone)]
            struct UnitStructWithCopy;
        };
        assert!(
            check_struct_implements_copy(input),
            "UnitStructWithCopy should implement Copy with #[derive(Copy)]"
        );

        // Test case 8: Tuple struct with #[derive(Copy)]
        let input: syn::DeriveInput = parse_quote! {
            #[derive(Copy, Clone)]
            struct TupleStruct(u32, bool, char);
        };
        assert!(
            check_struct_implements_copy(input),
            "TupleStruct should implement Copy with #[derive(Copy)]"
        );

        // Test case 9: Multiple derives including Copy
        let input: syn::DeriveInput = parse_quote! {
            #[derive(Debug, PartialEq, Copy, Clone)]
            struct MultipleDerivesStruct {
                a: u8,
            }
        };
        assert!(
            check_struct_implements_copy(input),
            "MultipleDerivesStruct should implement Copy with #[derive(Copy)]"
        );
    }
}
