use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::{utils, z_struct::FieldType};

/// Configuration system for zero-copy initialization
///
/// This module provides functionality to generate configuration structs and
/// initialization logic for zero-copy structures with Vec and Option fields.

// Note: The ZeroCopyInitMut trait is defined in the main zero-copy crate
// This module only contains helper functions for the derive macro
/// Helper functions for FieldType to support configuration
/// Determine if this field type requires configuration for initialization
pub fn requires_config(field_type: &FieldType) -> bool {
    match field_type {
        // Vec types always need length configuration
        FieldType::VecU8(_) | FieldType::VecCopy(_, _) | FieldType::VecNonCopy(_, _) => true,
        // Option types need Some/None configuration
        FieldType::Option(_, _) => true,
        // Fixed-size types don't need configuration
        FieldType::Array(_, _)
        | FieldType::Pubkey(_)
        | FieldType::IntegerU64(_)
        | FieldType::IntegerU32(_)
        | FieldType::IntegerU16(_)
        | FieldType::IntegerU8(_)
        | FieldType::Bool(_)
        | FieldType::CopyU8Bool(_)
        | FieldType::Copy(_, _) => false,
        // NonCopy types might need configuration if they contain Vec/Option
        FieldType::NonCopy(_, _) => true, // Conservative: assume they need config
    }
}

/// Generate the config type for this field
pub fn config_type(field_type: &FieldType) -> TokenStream {
    match field_type {
        // Simple Vec types: just need length
        FieldType::VecU8(_) => quote! { u32 },
        FieldType::VecCopy(_, _) => quote! { u32 },

        // Complex Vec types: need config for each element
        FieldType::VecNonCopy(_, vec_type) => {
            if let Some(inner_type) = utils::get_vec_inner_type(vec_type) {
                quote! { Vec<<#inner_type as light_zero_copy::ZeroCopyInitMut>::Config> }
            } else {
                panic!("Could not determine inner type for VecNonCopy config");
            }
        }

        // Option types: delegate to the Option's Config type
        FieldType::Option(_, option_type) => {
            quote! { <#option_type as light_zero_copy::ZeroCopyInitMut>::Config }
        }

        // Fixed-size types don't need configuration
        FieldType::Array(_, _)
        | FieldType::Pubkey(_)
        | FieldType::IntegerU64(_)
        | FieldType::IntegerU32(_)
        | FieldType::IntegerU16(_)
        | FieldType::IntegerU8(_)
        | FieldType::Bool(_)
        | FieldType::CopyU8Bool(_)
        | FieldType::Copy(_, _) => quote! { () },

        // NonCopy types: delegate to their Config type
        FieldType::NonCopy(_, field_type) => {
            quote! { <#field_type as light_zero_copy::ZeroCopyInitMut>::Config }
        }
    }
}

/// Generate a configuration struct for a given struct
pub fn generate_config_struct(struct_name: &Ident, field_types: &[FieldType]) -> TokenStream {
    let config_name = quote::format_ident!("{}Config", struct_name);

    // Generate config fields only for fields that require configuration
    let config_fields: Vec<TokenStream> = field_types
        .iter()
        .filter(|field_type| requires_config(field_type))
        .map(|field_type| {
            let field_name = field_type.name();
            let config_type = config_type(field_type);
            quote! {
                pub #field_name: #config_type,
            }
        })
        .collect();

    if config_fields.is_empty() {
        // If no fields require configuration, create an empty config struct
        quote! {
            #[derive(Debug, Clone, PartialEq)]
            pub struct #config_name;
        }
    } else {
        quote! {
            #[derive(Debug, Clone, PartialEq)]
            pub struct #config_name {
                #(#config_fields)*
            }
        }
    }
}

/// Generate initialization logic for a field based on its configuration
pub fn generate_field_initialization(field_type: &FieldType) -> TokenStream {
    match field_type {
        FieldType::VecU8(field_name) => {
            quote! {
                let (#field_name, bytes) = light_zero_copy::ZeroCopySliceMut::<u8, u8, false>::new_at(
                    config.#field_name,
                    bytes
                )?;
            }
        }

        FieldType::VecCopy(field_name, inner_type) => {
            quote! {
                let (#field_name, bytes) = light_zero_copy::ZeroCopySliceMut::<u8, #inner_type, false>::new_at(
                    config.#field_name,
                    bytes
                )?;
            }
        }

        FieldType::VecNonCopy(field_name, _) => {
            quote! {
                let (#field_name, bytes) = Vec::with_capacity(config.#field_name.len());
                // TODO: Initialize each element with its config
                // This requires more complex logic for per-element initialization
            }
        }

        FieldType::Option(field_name, option_type) => {
            quote! {
                let (#field_name, bytes) = <#option_type as light_zero_copy::ZeroCopyInitMut>::new_zero_copy(bytes, config.#field_name)?;
            }
        }

        // Fixed-size types don't need special initialization logic
        FieldType::Array(_, _)
        | FieldType::Pubkey(_)
        | FieldType::IntegerU64(_)
        | FieldType::IntegerU32(_)
        | FieldType::IntegerU16(_)
        | FieldType::IntegerU8(_)
        | FieldType::Bool(_)
        | FieldType::CopyU8Bool(_)
        | FieldType::Copy(_, _) => {
            quote! {
                // Fixed-size fields will be initialized from the meta struct
            }
        }

        FieldType::NonCopy(field_name, field_type) => {
            quote! {
                let (#field_name, bytes) = <#field_type as light_zero_copy::ZeroCopyInitMut>::new_zero_copy(
                    bytes,
                    config.#field_name
                )?;
            }
        }
    }
}
