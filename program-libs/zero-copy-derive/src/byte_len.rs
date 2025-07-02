use proc_macro2::TokenStream;
use quote::quote;
use syn::{Field, Ident};

use crate::{
    utils,
    z_struct::{analyze_struct_fields, FieldType},
};

/// Generates byte_len implementation for structs
///
/// RULES AND EXCEPTIONS FROM borsh_mut.rs:
///
/// DEFAULT RULE: Call byte_len() on each field and sum the results
///
/// EXCEPTIONS:
/// 1. Boolean fields: Use core::mem::size_of::<u8>() (1 byte) instead of byte_len()
///    * See line 97 where booleans use a special case
///
/// NOTES ON TYPE-SPECIFIC IMPLEMENTATIONS:
/// * Primitive types: self.field.byte_len() delegates to size_of::<T>()
///   - u8, u16, u32, u64, etc. all use size_of::<T>() in their implementations
///   - See implementations in lines 88-90, 146-148, and macro in lines 135-151
///
/// * Arrays [T; N]: use size_of::<Self>() in implementation (line 41)
///
/// * Vec<T>: 4 bytes for length prefix + sum of byte_len() for each element
///   - The Vec implementation in line 131 is: 4 + self.iter().map(|t| t.byte_len()).sum::<usize>()
///   - Special case in Struct4 (line 650-657): explicitly sums the byte_len of each item
///
/// * VecU8<T>: Uses 1 byte for length prefix instead of regular Vec's 4 bytes
///   - Implementation in line 205 shows: 1 + size_of::<T>()
///
/// * Option<T>: 1 byte for discriminator + value's byte_len if Some, or just 1 byte if None
///   - See implementation in lines 66-72
///
/// * Fixed-size types: Generally implement as their own fixed size
///   - Pubkey (line 45-46): hard-coded as 32 bytes
pub fn generate_byte_len_impl<'a>(
    _name: &Ident,
    meta_fields: &'a [&'a Field],
    struct_fields: &'a [&'a Field],
) -> TokenStream {
    let field_types = analyze_struct_fields(struct_fields);

    // Generate statements for calculating byte_len for each field
    let meta_byte_len = if !meta_fields.is_empty() {
        meta_fields
            .iter()
            .map(|field| {
                let field_name = &field.ident;
                // Handle boolean fields specially by using size_of instead of byte_len
                if utils::is_bool_type(&field.ty) {
                    quote! { core::mem::size_of::<u8>() }
                } else {
                    quote! { self.#field_name.byte_len() }
                }
            })
            .reduce(|acc, item| {
                quote! { #acc + #item }
            })
    } else {
        None
    };

    // Generate byte_len calculations for struct fields
    // Default rule: Use self.field.byte_len() for all fields
    // Exception: Use core::mem::size_of::<u8>() for boolean fields
    let struct_byte_len = field_types.into_iter().map(|field_type| {
        match field_type {
            // Exception 1: Booleans use size_of::<u8>() directly
            FieldType::Bool(_) | FieldType::CopyU8Bool(_) => {
                quote! { core::mem::size_of::<u8>() }
            }
            // All other types delegate to their own byte_len implementation
            FieldType::VecU8(field_name)
            | FieldType::VecCopy(field_name, _)
            | FieldType::VecNonCopy(field_name, _)
            | FieldType::Array(field_name, _)
            | FieldType::Option(field_name, _)
            | FieldType::Pubkey(field_name)
            | FieldType::IntegerU64(field_name)
            | FieldType::IntegerU32(field_name)
            | FieldType::IntegerU16(field_name)
            | FieldType::IntegerU8(field_name)
            | FieldType::Copy(field_name, _)
            | FieldType::NonCopy(field_name, _) => {
                quote! { self.#field_name.byte_len() }
            }
        }
    });

    // Combine meta fields and struct fields for total byte_len calculation
    let combined_byte_len = match meta_byte_len {
        Some(meta) => {
            let struct_bytes = struct_byte_len.fold(quote!(), |acc, item| {
                if acc.is_empty() {
                    item
                } else {
                    quote! { #acc + #item }
                }
            });

            if struct_bytes.is_empty() {
                meta
            } else {
                quote! { #meta + #struct_bytes }
            }
        }
        None => struct_byte_len.fold(quote!(), |acc, item| {
            if acc.is_empty() {
                item
            } else {
                quote! { #acc + #item }
            }
        }),
    };

    // Generate the final implementation
    quote! {
        fn byte_len(&self) -> usize {
            #combined_byte_len
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::format_ident;
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_generate_byte_len_simple() {
        let name = format_ident!("TestStruct");

        let field1: Field = parse_quote!(pub a: u8);
        let field2: Field = parse_quote!(pub b: u16);

        let meta_fields = vec![&field1, &field2];
        let struct_fields: Vec<&Field> = vec![];

        let result = generate_byte_len_impl(&name, &meta_fields, &struct_fields);
        let result_str = result.to_string();

        assert!(result_str.contains("fn byte_len (& self) -> usize"));
        assert!(result_str.contains("self . a . byte_len () + self . b . byte_len ()"));
    }

    #[test]
    fn test_generate_byte_len_with_vec() {
        let name = format_ident!("TestStruct");

        let field1: Field = parse_quote!(pub a: u8);
        let field2: Field = parse_quote!(pub vec: Vec<u8>);
        let field3: Field = parse_quote!(pub c: u32);

        let meta_fields = vec![&field1];
        let struct_fields = vec![&field2, &field3];

        let result = generate_byte_len_impl(&name, &meta_fields, &struct_fields);
        let result_str = result.to_string();

        assert!(result_str.contains("fn byte_len (& self) -> usize"));
        assert!(result_str.contains(
            "self . a . byte_len () + self . vec . byte_len () + self . c . byte_len ()"
        ));
    }

    #[test]
    fn test_generate_byte_len_with_option() {
        let name = format_ident!("TestStruct");

        let field1: Field = parse_quote!(pub a: u8);
        let field2: Field = parse_quote!(pub option: Option<u32>);

        let meta_fields = vec![&field1];
        let struct_fields = vec![&field2];

        let result = generate_byte_len_impl(&name, &meta_fields, &struct_fields);
        let result_str = result.to_string();

        assert!(result_str.contains("fn byte_len (& self) -> usize"));
        assert!(result_str.contains("self . a . byte_len () + self . option . byte_len ()"));
    }

    #[test]
    fn test_generate_byte_len_with_bool() {
        let name = format_ident!("TestStruct");

        let field1: Field = parse_quote!(pub a: u8);
        let field2: Field = parse_quote!(pub b: bool);

        let meta_fields = vec![&field1, &field2];
        let struct_fields: Vec<&Field> = vec![];

        let result = generate_byte_len_impl(&name, &meta_fields, &struct_fields);
        let result_str = result.to_string();

        assert!(result_str.contains("fn byte_len (& self) -> usize"));
        assert!(result_str.contains("self . a . byte_len () + core :: mem :: size_of :: < u8 > ()"));
    }
}
