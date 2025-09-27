use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, Data, DataEnum, DeriveInput, Field, Fields, FieldsNamed, Ident, Type, TypePath,
};

// Global cache for storing whether a struct implements Copy
lazy_static::lazy_static! {
    pub(crate) static ref COPY_IMPL_CACHE: Arc<Mutex<HashMap<String, bool>>> = Arc::new(Mutex::new(HashMap::new()));
}

/// Creates a unique cache key for a type using span information to avoid collisions
/// between types with the same name from different modules/locations
fn create_unique_type_key(ident: &Ident) -> String {
    format!("{}:{:?}", ident, ident.span())
}

/// Represents the type of input data (struct or enum)
pub enum InputType<'a> {
    Struct(&'a FieldsNamed),
    Enum(&'a DataEnum),
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

/// Process the derive input to extract information for both structs and enums
pub fn process_input_generic(
    input: &DeriveInput,
) -> syn::Result<(
    &Ident,             // Original name
    proc_macro2::Ident, // Z-name
    InputType<'_>,      // Input type (struct or enum)
)> {
    let name = &input.ident;
    let z_name = format_ident!("Z{}", name);

    // Populate the cache by checking if this struct implements Copy
    let _ = struct_implements_copy(input);

    let input_type = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => InputType::Struct(fields),
            _ => {
                return Err(syn::Error::new_spanned(
                    &data.fields,
                    "ZeroCopy only supports structs with named fields",
                ))
            }
        },
        Data::Enum(data) => InputType::Enum(data),
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "ZeroCopy only supports structs and enums",
            ))
        }
    };

    Ok((name, z_name, input_type))
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
                    "u16" => quote! { ::light_zero_copy::little_endian::U16 },
                    "u32" => quote! { ::light_zero_copy::little_endian::U32 },
                    "u64" => quote! { ::light_zero_copy::little_endian::U64 },
                    "i16" => quote! { ::light_zero_copy::little_endian::I16 },
                    "i32" => quote! { ::light_zero_copy::little_endian::I32 },
                    "i64" => quote! { ::light_zero_copy::little_endian::I64 },
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
        Type::Array(array) => {
            // Recursively convert the element type
            let elem = convert_to_zerocopy_type(&array.elem);
            let len = &array.len;
            quote! { [#elem; #len] }
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

/// Checks if a struct has a #[light_hasher] attribute
pub fn struct_has_light_hasher_attribute(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .any(|attr| attr.path().is_ident("light_hasher"))
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
    if let Ok(cache) = COPY_IMPL_CACHE.lock() {
        if let Some(implements_copy) = cache.get(&cache_key) {
            return *implements_copy;
        }
    }
    // If mutex is poisoned, we can still continue without cache

    // Check if the struct has a derive(Copy) attribute
    let implements_copy = struct_has_copy_derive(&input.attrs);

    // Cache the result (ignore if mutex is poisoned)
    if let Ok(mut cache) = COPY_IMPL_CACHE.lock() {
        cache.insert(cache_key, implements_copy);
    }

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
                if let Ok(cache) = COPY_IMPL_CACHE.lock() {
                    if let Some(implements_copy) = cache.get(&cache_key) {
                        return *implements_copy;
                    }
                }
                // If mutex is poisoned, continue without cache
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

/// Check if a type needs to use the ZeroCopyStructInner trait.
/// Arrays and primitive types can be used directly after type conversion,
/// while custom structs need to go through the trait's associated type.
pub fn needs_struct_inner_trait(ty: &Type) -> bool {
    // Arrays don't implement ZeroCopyStructInner - use directly
    if matches!(ty, Type::Array(_)) {
        return false;
    }

    // Primitive types and bool are used directly after conversion
    if is_primitive_integer(ty) || is_bool_type(ty) || is_pubkey_type(ty) {
        return false;
    }

    // All other types (custom structs) need the trait
    true
}

/// Check if a struct has #[repr(C)] attribute
pub fn has_repr_c_attribute(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("repr") {
            // Parse the repr attribute arguments
            // Convert tokens to string and check if it contains "C"
            // This handles both #[repr(C)] and #[repr(C, packed)] etc.
            let tokens = attr.meta.clone();
            if let syn::Meta::List(list) = tokens {
                // Convert tokens to string and check for "C"
                let tokens_str = list.tokens.to_string();
                // Split by comma and check each part
                for part in tokens_str.split(',') {
                    let trimmed = part.trim();
                    // Check if this part is exactly "C" (not part of another word)
                    if trimmed == "C" {
                        return true;
                    }
                }
            } else if let syn::Meta::Path(path) = tokens {
                // Handle #[repr(C)] without parentheses (though unlikely)
                return path.is_ident("C");
            }
            false
        } else {
            false
        }
    })
}

/// Validate that the input has #[repr(C)] attribute for memory layout safety
pub fn validate_repr_c_required(attrs: &[syn::Attribute], item_type: &str) -> syn::Result<()> {
    if !has_repr_c_attribute(attrs) {
        return Err(syn::Error::new_spanned(
            attrs.first().unwrap_or(&syn::parse_quote!(#[dummy])),
            format!(
                "{} requires #[repr(C)] attribute for memory layout safety. Add #[repr(C)] above the {} declaration.",
                item_type, item_type.to_lowercase()
            )
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::has_repr_c_attribute;

    #[test]
    fn test_repr_c_detection() {
        // Test single #[repr(C)]
        let input = quote! {
            #[repr(C)]
            struct Test {}
        };
        let parsed: syn::DeriveInput = syn::parse2(input).unwrap();
        assert!(
            has_repr_c_attribute(&parsed.attrs),
            "Should detect #[repr(C)]"
        );

        // Test #[repr(C, packed)]
        let input = quote! {
            #[repr(C, packed)]
            struct Test {}
        };
        let parsed: syn::DeriveInput = syn::parse2(input).unwrap();
        assert!(
            has_repr_c_attribute(&parsed.attrs),
            "Should detect C in #[repr(C, packed)]"
        );

        // Test #[repr(C, align(8))]
        let input = quote! {
            #[repr(C, align(8))]
            struct Test {}
        };
        let parsed: syn::DeriveInput = syn::parse2(input).unwrap();
        assert!(
            has_repr_c_attribute(&parsed.attrs),
            "Should detect C in #[repr(C, align(8))]"
        );

        // Test #[repr(packed, C)]
        let input = quote! {
            #[repr(packed, C)]
            struct Test {}
        };
        let parsed: syn::DeriveInput = syn::parse2(input).unwrap();
        assert!(
            has_repr_c_attribute(&parsed.attrs),
            "Should detect C in #[repr(packed, C)]"
        );

        // Test #[repr(packed)] without C
        let input = quote! {
            #[repr(packed)]
            struct Test {}
        };
        let parsed: syn::DeriveInput = syn::parse2(input).unwrap();
        assert!(
            !has_repr_c_attribute(&parsed.attrs),
            "Should not detect C in #[repr(packed)]"
        );

        // Test no repr attribute
        let input = quote! {
            struct Test {}
        };
        let parsed: syn::DeriveInput = syn::parse2(input).unwrap();
        assert!(
            !has_repr_c_attribute(&parsed.attrs),
            "Should not detect C without repr"
        );

        // Test #[repr(Rust)]
        let input = quote! {
            #[repr(Rust)]
            struct Test {}
        };
        let parsed: syn::DeriveInput = syn::parse2(input).unwrap();
        assert!(
            !has_repr_c_attribute(&parsed.attrs),
            "Should not detect C in #[repr(Rust)]"
        );
    }
}
