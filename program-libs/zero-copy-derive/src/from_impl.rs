use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Field, Ident, Type};

use crate::z_struct::{analyze_struct_fields, FieldType};

/// Generates code for the From<Z<StructName>> for StructName implementation
/// The `MUT` parameter controls whether to generate code for mutable or immutable references
pub fn generate_from_impl<const MUT: bool>(
    name: &Ident,
    z_struct_name: &Ident,
    meta_fields: &[&Field],
    struct_fields: &[&Field],
) -> TokenStream {
    let mut z_struct_name = z_struct_name.clone();

    if MUT {
        z_struct_name = format_ident!("{}Mut", z_struct_name);
    }

    let _z_struct_meta_name = if MUT {
        format_ident!("{}MetaMut", z_struct_name)
    } else {
        format_ident!("{}Meta", z_struct_name)
    };

    // Generate the conversion code for meta fields
    let meta_field_conversions = if !meta_fields.is_empty() {
        let field_types = analyze_struct_fields(meta_fields);
        let conversions = field_types.into_iter().map(|field_type| {
            match field_type {
                FieldType::IntegerU64(field_name) => {
                    quote! { #field_name: u64::from(value.__meta.#field_name), }
                }
                FieldType::IntegerU32(field_name) => {
                    quote! { #field_name: u32::from(value.__meta.#field_name), }
                }
                FieldType::IntegerU16(field_name) => {
                    quote! { #field_name: u16::from(value.__meta.#field_name), }
                }
                FieldType::IntegerU8(field_name) => {
                    quote! { #field_name: value.__meta.#field_name, }
                }
                FieldType::Bool(field_name) => {
                    quote! { #field_name: value.__meta.#field_name > 0, }
                }
                FieldType::Array(field_name, _) => {
                    // For arrays, just copy the value
                    quote! { #field_name: value.__meta.#field_name, }
                }
                FieldType::Pubkey(field_name) => {
                    quote! { #field_name: value.__meta.#field_name, }
                }
                _ => {
                    let field_name = field_type.name();
                    quote! { #field_name: value.__meta.#field_name.into(), }
                }
            }
        });
        conversions.collect::<Vec<_>>()
    } else {
        vec![]
    };

    // Generate the conversion code for struct fields
    let struct_field_conversions = if !struct_fields.is_empty() {
        let field_types = analyze_struct_fields(struct_fields);
        let conversions = field_types.into_iter().map(|field_type| {
            match field_type {
                FieldType::VecU8(field_name) => {
                    quote! { #field_name: value.#field_name.to_vec(), }
                }
                FieldType::VecCopy(field_name, _) => {
                    quote! { #field_name: value.#field_name.to_vec(), }
                }
                FieldType::VecNonCopy(field_name, _) => {
                    // For non-copy vectors, clone each element directly
                    // We need to convert into() for Zstructs
                    quote! {
                        #field_name: {
                            value.#field_name.iter().map(|item| (*item).clone().into()).collect()
                        },
                    }
                }
                FieldType::Array(field_name, _) => {
                    // For arrays, just copy the value
                    quote! { #field_name: *value.#field_name, }
                }
                FieldType::Option(field_name, field_type) => {
                    fn extract_inner(s: &str) -> Option<&str> {
                        s.strip_prefix("Option <")?.strip_suffix(">")
                    }
                    use quote::ToTokens;
                    let string = field_type.to_token_stream().to_string();
                    println!("option string {}", string);
                    let cleaned_type = extract_inner(&string).unwrap();
                    let field_type = syn::parse_str::<Type>(cleaned_type).unwrap();
                    // For Option types, use a direct copy of the value when possible
                    quote! {
                        #field_name: if value.#field_name.is_some() {
                            // Create a clone of the Some value - for compressed proofs and other structs
                            // For instruction_data.rs, we just need to clone the value directly
                            Some((#field_type::from(*value.#field_name.as_ref().unwrap()).clone()))
                        } else {
                            None
                        },
                    }
                }
                FieldType::Pubkey(field_name) => {
                    quote! { #field_name: *value.#field_name, }
                }
                FieldType::Bool(field_name) => {
                    if MUT {
                        quote! { #field_name: *value.#field_name > 0, }
                    } else {
                        quote! { #field_name: value.#field_name > 0, }
                    }
                }
                FieldType::CopyU8Bool(field_name) => {
                    quote! { #field_name: value.#field_name > 0, }
                }
                FieldType::IntegerU64(field_name) => {
                    quote! { #field_name: u64::from(*value.#field_name), }
                }
                FieldType::IntegerU32(field_name) => {
                    quote! { #field_name: u32::from(*value.#field_name), }
                }
                FieldType::IntegerU16(field_name) => {
                    quote! { #field_name: u16::from(*value.#field_name), }
                }
                FieldType::IntegerU8(field_name) => {
                    if MUT {
                        quote! { #field_name: *value.#field_name, }
                    } else {
                        quote! { #field_name: value.#field_name, }
                    }
                }
                FieldType::Copy(field_name, _) => {
                    quote! { #field_name: value.#field_name, }
                }
                FieldType::OptionU64(field_name) => {
                    quote! { #field_name: value.#field_name.as_ref().map(|x| u64::from(**x)), }
                }
                FieldType::OptionU32(field_name) => {
                    quote! { #field_name: value.#field_name.as_ref().map(|x| u32::from(**x)), }
                }
                FieldType::OptionU16(field_name) => {
                    quote! { #field_name: value.#field_name.as_ref().map(|x| u16::from(**x)), }
                }
                FieldType::NonCopy(field_name, field_type) => {
                    // For complex non-copy types, dereference and clone directly
                    quote! { #field_name: #field_type::from(&value.#field_name), }
                }
            }
        });
        conversions.collect::<Vec<_>>()
    } else {
        vec![]
    };

    // Combine all the field conversions
    let all_field_conversions = [meta_field_conversions, struct_field_conversions].concat();

    // Return the final From implementation without generic From implementations
    quote! {
        impl<'a> From<#z_struct_name<'a>> for #name {
            fn from(value: #z_struct_name<'a>) -> Self {
                Self {
                    #(#all_field_conversions)*
                }
            }
        }

        impl<'a> From<&#z_struct_name<'a>> for #name {
            fn from(value: &#z_struct_name<'a>) -> Self {
                Self {
                    #(#all_field_conversions)*
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::format_ident;
    use syn::{parse_quote, Field};

    use super::*;

    #[test]
    fn test_generate_from_impl() {
        // Create a struct for testing
        let name = format_ident!("TestStruct");
        let z_struct_name = format_ident!("ZTestStruct");

        // Create some test fields
        let field_a: Field = parse_quote!(pub a: u8);
        let field_b: Field = parse_quote!(pub b: u16);
        let field_c: Field = parse_quote!(pub c: Vec<u8>);

        // Split into meta and struct fields
        let meta_fields = vec![&field_a, &field_b];
        let struct_fields = vec![&field_c];

        // Generate the implementation
        let result =
            generate_from_impl::<false>(&name, &z_struct_name, &meta_fields, &struct_fields);

        // Convert to string for testing
        let result_str = result.to_string();

        // Check that the implementation contains required elements
        println!("Generated code: {}", result_str);
        assert!(result_str.contains("impl < 'a > From < ZTestStruct < 'a >> for TestStruct"));

        // Check field handling
        assert!(result_str.contains("a :")); // For u8 fields
        assert!(result_str.contains("b :")); // For u16 fields
        assert!(result_str.contains("c :")); // For Vec<u8> fields
    }

    #[test]
    fn test_generate_from_impl_mut() {
        // Create a struct for testing
        let name = format_ident!("TestStruct");
        let z_struct_name = format_ident!("ZTestStruct");

        // Create some test fields
        let field_a: Field = parse_quote!(pub a: u8);
        let field_b: Field = parse_quote!(pub b: bool);
        let field_c: Field = parse_quote!(pub c: Option<u32>);

        // Split into meta and struct fields
        let meta_fields = vec![&field_a, &field_b];
        let struct_fields = vec![&field_c];

        // Generate the implementation for mutable version
        let result =
            generate_from_impl::<true>(&name, &z_struct_name, &meta_fields, &struct_fields);

        // Convert to string for testing
        let result_str = result.to_string();

        // Check that the implementation contains required elements
        println!("Generated code (mut): {}", result_str);
        assert!(result_str.contains("impl < 'a > From < ZTestStructMut < 'a >> for TestStruct"));

        // Check field handling
        assert!(result_str.contains("a :")); // For u8 fields
        assert!(result_str.contains("b :")); // For bool fields
        assert!(result_str.contains("c :")); // For Option fields
    }
}
