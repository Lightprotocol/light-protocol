use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::{utils, z_struct::FieldType};

/// Configuration system for zero-copy initialization
///
/// This module provides functionality to generate configuration structs and
/// initialization logic for zero-copy structures with Vec and Option fields.
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
        // Option integer types need config to determine if they're enabled
        FieldType::OptionU64(_) | FieldType::OptionU32(_) | FieldType::OptionU16(_) => true,
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
                quote! { Vec<<#inner_type as light_zero_copy::init_mut::ZeroCopyNew<'static>>::Config> }
            } else {
                panic!("Could not determine inner type for VecNonCopy config");
            }
        }

        // Option types: delegate to the Option's Config type
        FieldType::Option(_, option_type) => {
            quote! { <#option_type as light_zero_copy::init_mut::ZeroCopyNew<'static>>::Config }
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

        // Option integer types: use bool config to determine if enabled
        FieldType::OptionU64(_) | FieldType::OptionU32(_) | FieldType::OptionU16(_) => {
            quote! { bool }
        }

        // NonCopy types: delegate to their Config type (Config is typically 'static)
        FieldType::NonCopy(_, field_type) => {
            quote! { <#field_type as light_zero_copy::init_mut::ZeroCopyNew<'static>>::Config }
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
                // Initialize the length prefix but don't use the returned ZeroCopySliceMut
                {
                    light_zero_copy::slice_mut::ZeroCopySliceMutBorsh::<u8>::new_at(
                        config.#field_name.into(),
                        bytes
                    )?;
                }
                // Split off the length prefix (4 bytes) and get the slice
                let (_, bytes) = bytes.split_at_mut(4);
                let (#field_name, bytes) = bytes.split_at_mut(config.#field_name as usize);
            }
        }

        FieldType::VecCopy(field_name, inner_type) => {
            quote! {
                let (#field_name, bytes) = light_zero_copy::slice_mut::ZeroCopySliceMutBorsh::<#inner_type>::new_at(
                    config.#field_name.into(),
                    bytes
                )?;
            }
        }

        FieldType::VecNonCopy(field_name, vec_type) => {
            quote! {
                let (#field_name, bytes) = <#vec_type as light_zero_copy::init_mut::ZeroCopyNew<'a>>::new_zero_copy(
                    bytes,
                    config.#field_name
                )?;
            }
        }

        FieldType::Option(field_name, option_type) => {
            quote! {
                let (#field_name, bytes) = <#option_type as light_zero_copy::init_mut::ZeroCopyNew<'a>>::new_zero_copy(bytes, config.#field_name)?;
            }
        }

        // Fixed-size types that are struct fields (not meta fields) need initialization with () config
        FieldType::IntegerU64(field_name) => {
            quote! {
                let (#field_name, bytes) = light_zero_copy::Ref::<
                    &'a mut [u8],
                    light_zero_copy::little_endian::U64
                >::from_prefix(bytes)?;
            }
        }

        FieldType::IntegerU32(field_name) => {
            quote! {
                let (#field_name, bytes) = light_zero_copy::Ref::<
                    &'a mut [u8],
                    light_zero_copy::little_endian::U32
                >::from_prefix(bytes)?;
            }
        }

        FieldType::IntegerU16(field_name) => {
            quote! {
                let (#field_name, bytes) = light_zero_copy::Ref::<
                    &'a mut [u8],
                    light_zero_copy::little_endian::U16
                >::from_prefix(bytes)?;
            }
        }

        FieldType::IntegerU8(field_name) => {
            quote! {
                let (#field_name, bytes) = light_zero_copy::Ref::<&mut [u8], u8>::from_prefix(bytes)?;
            }
        }

        FieldType::Bool(field_name) => {
            quote! {
                let (#field_name, bytes) = light_zero_copy::Ref::<&mut [u8], u8>::from_prefix(bytes)?;
            }
        }

        // Array fields that are struct fields (come after Vec/Option)
        FieldType::Array(field_name, array_type) => {
            quote! {
                let (#field_name, bytes) = light_zero_copy::Ref::<
                    &'a mut [u8],
                    #array_type
                >::from_prefix(bytes)?;
            }
        }

        FieldType::Pubkey(field_name) => {
            quote! {
                let (#field_name, bytes) = light_zero_copy::Ref::<
                    &'a mut [u8],
                    Pubkey
                >::from_prefix(bytes)?;
            }
        }

        // Types that are truly meta fields (shouldn't reach here for struct fields)
        FieldType::CopyU8Bool(_) | FieldType::Copy(_, _) => {
            quote! {
                // Should not reach here for struct fields - these should be meta fields
            }
        }

        FieldType::OptionU64(field_name) => {
            quote! {
                let (#field_name, bytes) = <Option<u64> as light_zero_copy::init_mut::ZeroCopyNew>::new_zero_copy(
                    bytes,
                    (config.#field_name, ())
                )?;
            }
        }

        FieldType::OptionU32(field_name) => {
            quote! {
                let (#field_name, bytes) = <Option<u32> as light_zero_copy::init_mut::ZeroCopyNew>::new_zero_copy(
                    bytes,
                    (config.#field_name, ())
                )?;
            }
        }

        FieldType::OptionU16(field_name) => {
            quote! {
                let (#field_name, bytes) = <Option<u16> as light_zero_copy::init_mut::ZeroCopyNew>::new_zero_copy(
                    bytes,
                    (config.#field_name, ())
                )?;
            }
        }

        FieldType::NonCopy(field_name, field_type) => {
            quote! {
                let (#field_name, bytes) = <#field_type as light_zero_copy::init_mut::ZeroCopyNew<'a>>::new_zero_copy(
                    bytes,
                    config.#field_name
                )?;
            }
        }
    }
}

/// Generate byte length calculation for a field based on its configuration
pub fn generate_byte_len_calculation(field_type: &FieldType) -> TokenStream {
    match field_type {
        // Vec types that require configuration
        FieldType::VecU8(field_name) => {
            quote! {
                (4 + config.#field_name as usize) // 4 bytes for length + actual data
            }
        }

        FieldType::VecCopy(field_name, inner_type) => {
            quote! {
                (4 + (config.#field_name as usize * core::mem::size_of::<#inner_type>()))
            }
        }

        FieldType::VecNonCopy(field_name, vec_type) => {
            quote! {
                <#vec_type as light_zero_copy::init_mut::ZeroCopyNew<'static>>::byte_len(&config.#field_name)
            }
        }

        // Option types
        FieldType::Option(field_name, option_type) => {
            quote! {
                <#option_type as light_zero_copy::init_mut::ZeroCopyNew<'static>>::byte_len(&config.#field_name)
            }
        }

        FieldType::OptionU64(field_name) => {
            quote! {
                <Option<u64> as light_zero_copy::init_mut::ZeroCopyNew<'static>>::byte_len(&(config.#field_name, ()))
            }
        }

        FieldType::OptionU32(field_name) => {
            quote! {
                <Option<u32> as light_zero_copy::init_mut::ZeroCopyNew<'static>>::byte_len(&(config.#field_name, ()))
            }
        }

        FieldType::OptionU16(field_name) => {
            quote! {
                <Option<u16> as light_zero_copy::init_mut::ZeroCopyNew<'static>>::byte_len(&(config.#field_name, ()))
            }
        }

        // Fixed-size types don't need configuration and have known sizes
        FieldType::IntegerU64(_) => {
            quote! {
                core::mem::size_of::<light_zero_copy::little_endian::U64>()
            }
        }

        FieldType::IntegerU32(_) => {
            quote! {
                core::mem::size_of::<light_zero_copy::little_endian::U32>()
            }
        }

        FieldType::IntegerU16(_) => {
            quote! {
                core::mem::size_of::<light_zero_copy::little_endian::U16>()
            }
        }

        FieldType::IntegerU8(_) => {
            quote! {
                core::mem::size_of::<u8>()
            }
        }

        FieldType::Bool(_) => {
            quote! {
                core::mem::size_of::<u8>()  // bool is serialized as u8
            }
        }

        FieldType::Array(_, array_type) => {
            quote! {
                core::mem::size_of::<#array_type>()
            }
        }

        FieldType::Pubkey(_) => {
            quote! {
                32  // Pubkey is always 32 bytes
            }
        }

        // Meta field types (should not appear in struct fields, but handle gracefully)
        FieldType::CopyU8Bool(_) => {
            quote! {
                core::mem::size_of::<u8>()
            }
        }

        FieldType::Copy(_, field_type) => {
            quote! {
                core::mem::size_of::<#field_type>()
            }
        }

        FieldType::NonCopy(field_name, field_type) => {
            quote! {
                <#field_type as light_zero_copy::init_mut::ZeroCopyNew<'static>>::byte_len(&config.#field_name)
            }
        }
    }
}
