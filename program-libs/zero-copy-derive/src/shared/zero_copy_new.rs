use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Ident;

use crate::shared::{
    utils,
    z_struct::{analyze_struct_fields, FieldType},
};

/// Generate ZeroCopyNew implementation with new_at method for a struct
pub fn generate_init_mut_impl(
    struct_name: &syn::Ident,
    meta_fields: &[&syn::Field],
    struct_fields: &[&syn::Field],
) -> syn::Result<proc_macro2::TokenStream> {
    let config_name = quote::format_ident!("{}Config", struct_name);
    let z_meta_name = quote::format_ident!("Z{}MetaMut", struct_name);
    let z_struct_mut_name = quote::format_ident!("Z{}Mut", struct_name);

    // Use the pre-separated fields from utils::process_fields (consistent with other derives)
    let struct_field_types = analyze_struct_fields(struct_fields)?;

    // Generate field initialization code for struct fields only (meta fields are part of __meta)
    let field_initializations: Result<Vec<proc_macro2::TokenStream>, syn::Error> =
        struct_field_types
            .iter()
            .map(|field_type| generate_field_initialization(field_type))
            .collect();
    let field_initializations = field_initializations?;

    // Generate struct construction - only include struct fields that were initialized
    // Meta fields are accessed via __meta.field_name in the generated ZStruct
    let struct_field_names: Vec<proc_macro2::TokenStream> = struct_field_types
        .iter()
        .map(|field_type| {
            let field_name = field_type.name();
            quote! { #field_name, }
        })
        .collect();

    // Check if there are meta fields to determine whether to include __meta
    let has_meta_fields = !meta_fields.is_empty();

    let meta_initialization = if has_meta_fields {
        quote! {
            // Handle the meta struct (fixed-size fields at the beginning)
            let (__meta, bytes) = Ref::<&mut [u8], #z_meta_name>::from_prefix(bytes)?;
        }
    } else {
        quote! {
            // No meta fields, skip meta struct initialization
        }
    };

    let struct_construction = if has_meta_fields {
        quote! {
            let result = #z_struct_mut_name {
                __meta,
                #(#struct_field_names)*
            };
        }
    } else {
        quote! {
            let result = #z_struct_mut_name {
                #(#struct_field_names)*
            };
        }
    };

    // Generate byte_len calculation for each field type
    let byte_len_calculations: Result<Vec<proc_macro2::TokenStream>, syn::Error> =
        struct_field_types
            .iter()
            .map(|field_type| generate_byte_len_calculation(field_type))
            .collect();
    let byte_len_calculations = byte_len_calculations?;

    // Calculate meta size if there are meta fields
    let meta_size_calculation = if has_meta_fields {
        quote! {
            core::mem::size_of::<#z_meta_name>()
        }
    } else {
        quote! { 0 }
    };

    let result = quote! {
        impl<'a> light_zero_copy::init_mut::ZeroCopyNew<'a> for #struct_name {
            type Config = #config_name;
            type Output = <Self as light_zero_copy::borsh_mut::DeserializeMut<'a>>::Output;

            fn byte_len(config: &Self::Config) -> usize {
                #meta_size_calculation #(+ #byte_len_calculations)*
            }

            fn new_zero_copy(
                bytes: &'a mut [u8],
                config: Self::Config,
            ) -> Result<(Self::Output, &'a mut [u8]), light_zero_copy::errors::ZeroCopyError> {
                use zerocopy::Ref;

                #meta_initialization

                #(#field_initializations)*

                #struct_construction

                Ok((result, bytes))
            }
        }
    };
    Ok(result)
}

// Configuration system functions moved from config.rs

/// Determine if this field type requires configuration for initialization
pub fn requires_config(field_type: &FieldType) -> bool {
    match field_type {
        // Vec types always need length configuration
        FieldType::VecU8(_) | FieldType::VecCopy(_, _) | FieldType::VecDynamicZeroCopy(_, _) => {
            true
        }
        // Option types need Some/None configuration
        FieldType::Option(_, _) => true,
        // Fixed-size types don't need configuration
        FieldType::Array(_, _)
        | FieldType::Pubkey(_)
        | FieldType::Primitive(_, _)
        | FieldType::Copy(_, _) => false,
        // DynamicZeroCopy types might need configuration if they contain Vec/Option
        FieldType::DynamicZeroCopy(_, _) => true, // Conservative: assume they need config
        // Option integer types need config to determine if they're enabled
        FieldType::OptionU64(_) | FieldType::OptionU32(_) | FieldType::OptionU16(_) => true,
    }
}

/// Generate the config type for this field
pub fn config_type(field_type: &FieldType) -> syn::Result<TokenStream2> {
    let result = match field_type {
        // Simple Vec types: just need length
        FieldType::VecU8(_) => quote! { u32 },
        FieldType::VecCopy(_, _) => quote! { u32 },

        // Complex Vec types: need config for each element
        FieldType::VecDynamicZeroCopy(_, vec_type) => {
            if let Some(inner_type) = utils::get_vec_inner_type(vec_type) {
                quote! { Vec<<#inner_type as light_zero_copy::init_mut::ZeroCopyNew<'static>>::Config> }
            } else {
                return Err(syn::Error::new_spanned(
                    vec_type,
                    "Could not determine inner type for VecDynamicZeroCopy config",
                ));
            }
        }

        // Option types: delegate to the Option's Config type
        FieldType::Option(_, option_type) => {
            quote! { <#option_type as light_zero_copy::init_mut::ZeroCopyNew<'static>>::Config }
        }

        // Fixed-size types don't need configuration
        FieldType::Array(_, _)
        | FieldType::Pubkey(_)
        | FieldType::Primitive(_, _)
        | FieldType::Copy(_, _) => quote! { () },

        // Option integer types: use bool config to determine if enabled
        FieldType::OptionU64(_) | FieldType::OptionU32(_) | FieldType::OptionU16(_) => {
            quote! { bool }
        }

        // DynamicZeroCopy types: delegate to their Config type (Config is typically 'static)
        FieldType::DynamicZeroCopy(_, field_type) => {
            let field_type = utils::convert_to_zerocopy_type(field_type);
            quote! { <#field_type as light_zero_copy::init_mut::ZeroCopyNew<'static>>::Config }
        }
    };
    Ok(result)
}

/// Generate a configuration struct for a given struct
pub fn generate_config_struct(
    struct_name: &Ident,
    field_types: &[FieldType],
) -> syn::Result<TokenStream2> {
    let config_name = quote::format_ident!("{}Config", struct_name);

    // Generate config fields only for fields that require configuration
    let config_fields: Result<Vec<TokenStream2>, syn::Error> = field_types
        .iter()
        .filter(|field_type| requires_config(field_type))
        .map(|field_type| -> syn::Result<TokenStream2> {
            let field_name = field_type.name();
            let config_type = config_type(field_type)?;
            Ok(quote! {
                pub #field_name: #config_type,
            })
        })
        .collect();
    let config_fields = config_fields?;

    let result = if config_fields.is_empty() {
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
    };
    Ok(result)
}

/// Generate initialization logic for a field based on its configuration
pub fn generate_field_initialization(field_type: &FieldType) -> syn::Result<TokenStream2> {
    let result = match field_type {
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

        FieldType::VecDynamicZeroCopy(field_name, vec_type)
        | FieldType::DynamicZeroCopy(field_name, vec_type)
        | FieldType::Option(field_name, vec_type) => {
            quote! {
                let (#field_name, bytes) = <#vec_type as light_zero_copy::init_mut::ZeroCopyNew<'a>>::new_zero_copy(
                    bytes,
                    config.#field_name
                )?;
            }
        }

        FieldType::OptionU64(field_name)
        | FieldType::OptionU32(field_name)
        | FieldType::OptionU16(field_name) => {
            let option_type = match field_type {
                FieldType::OptionU64(_) => quote! { Option<u64> },
                FieldType::OptionU32(_) => quote! { Option<u32> },
                FieldType::OptionU16(_) => quote! { Option<u16> },
                _ => unreachable!(),
            };
            quote! {
                let (#field_name, bytes) = <#option_type as light_zero_copy::init_mut::ZeroCopyNew>::new_zero_copy(
                    bytes,
                    (config.#field_name, ())
                )?;
            }
        }

        // Fixed-size types that are struct fields (not meta fields) need initialization with () config
        FieldType::Primitive(field_name, field_type) => {
            quote! {
                let (#field_name, bytes) = <#field_type as light_zero_copy::borsh_mut::DeserializeMut>::zero_copy_at_mut(bytes)?;
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

        FieldType::Copy(field_name, field_type) => {
            quote! {
                let (#field_name, bytes) = <#field_type as light_zero_copy::init_mut::ZeroCopyNew>::new_zero_copy(bytes)?;
            }
        }
    };
    Ok(result)
}

/// Generate byte length calculation for a field based on its configuration
pub fn generate_byte_len_calculation(field_type: &FieldType) -> syn::Result<TokenStream2> {
    let result = match field_type {
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

        FieldType::VecDynamicZeroCopy(field_name, vec_type) => {
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
        FieldType::Primitive(_, field_type) => {
            let zerocopy_type = utils::convert_to_zerocopy_type(field_type);
            quote! {
                core::mem::size_of::<#zerocopy_type>()
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
        FieldType::Copy(_, field_type) => {
            quote! {
                core::mem::size_of::<#field_type>()
            }
        }

        FieldType::DynamicZeroCopy(field_name, field_type) => {
            quote! {
                <#field_type as light_zero_copy::init_mut::ZeroCopyNew<'static>>::byte_len(&config.#field_name)
            }
        }
    };
    Ok(result)
}
