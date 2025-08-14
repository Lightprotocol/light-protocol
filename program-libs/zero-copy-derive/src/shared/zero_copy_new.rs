use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Ident;

use crate::shared::{
    utils,
    z_struct::{analyze_struct_fields, FieldType},
};

/// Detailed analysis of field complexity for better error messages and optimization
#[derive(Debug, Clone)]
pub enum FieldStrategy {
    FixedSize,
    DynamicKnownBounds,
}

/// Generate ZeroCopyNew implementation with new_at method for a struct
/// Handles both complex configs (with fields) and unit configs (for fixed-size structs)
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
            .map(|field_type| generate_field_initialization_for_config(field_type))
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
            let (__meta, __remaining_bytes) = Ref::<&mut [u8], #z_meta_name>::from_prefix(__remaining_bytes)?;
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
            ::core::mem::size_of::<#z_meta_name>()
        }
    } else {
        quote! { 0 }
    };

    let result = quote! {
        impl<'a> ::light_zero_copy::traits::ZeroCopyNew<'a> for #struct_name {
            type ZeroCopyConfig = #config_name;
            type Output = <Self as ::light_zero_copy::traits::ZeroCopyAtMut<'a>>::ZeroCopyAtMut;

            fn byte_len(config: &Self::ZeroCopyConfig) -> Result<usize, ::light_zero_copy::errors::ZeroCopyError> {
                let mut total: usize = #meta_size_calculation;
                #(
                    let field_len = match #byte_len_calculations {
                        Ok(len) => len,
                        Err(e) => return Err(e),
                    };
                    total = match total.checked_add(field_len) {
                        Some(new_total) => new_total,
                        None => return Err(::light_zero_copy::errors::ZeroCopyError::Size),
                    };
                )*
                Ok(total)
            }

            fn new_zero_copy(
                __remaining_bytes: &'a mut [u8],
                config: Self::ZeroCopyConfig,
            ) -> Result<(Self::Output, &'a mut [u8]), ::light_zero_copy::errors::ZeroCopyError> {
                use ::zerocopy::Ref;

                #meta_initialization

                #(#field_initializations)*

                #struct_construction

                Ok((result, __remaining_bytes))
            }
        }
    };
    Ok(result)
}

/// Analyze field strategy with simplified categorization
pub fn analyze_field_strategy(field_type: &FieldType) -> FieldStrategy {
    match field_type {
        // Fixed-size fields that don't require configuration
        FieldType::Primitive(_, _)
        | FieldType::Copy(_, _)
        | FieldType::Pubkey(_)
        | FieldType::Array(_, _) => FieldStrategy::FixedSize,

        // Dynamic fields that require configuration
        FieldType::VecU8(_)
        | FieldType::VecCopy(_, _)
        | FieldType::VecDynamicZeroCopy(_, _)
        | FieldType::Option(_, _)
        | FieldType::OptionU64(_)
        | FieldType::OptionU32(_)
        | FieldType::OptionU16(_)
        | FieldType::OptionArray(_, _)
        | FieldType::DynamicZeroCopy(_, _) => FieldStrategy::DynamicKnownBounds,
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
                quote! { Vec<<#inner_type as ::light_zero_copy::traits::ZeroCopyNew<'static>>::ZeroCopyConfig> }
            } else {
                return Err(syn::Error::new_spanned(
                    vec_type,
                    "Could not determine inner type for VecDynamicZeroCopy config",
                ));
            }
        }

        // Option types: delegate to the Option's Config type
        FieldType::Option(_, option_type) => {
            quote! { <#option_type as ::light_zero_copy::traits::ZeroCopyNew<'static>>::ZeroCopyConfig }
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

        // Option<[T; N]> needs the full Option config type
        FieldType::OptionArray(_, array_type) => {
            let array_type_zerocopy = utils::convert_to_zerocopy_type(array_type);
            quote! { <Option<#array_type_zerocopy> as ::light_zero_copy::traits::ZeroCopyNew<'static>>::ZeroCopyConfig }
        }

        // DynamicZeroCopy types: delegate to their Config type (Config is typically 'static)
        FieldType::DynamicZeroCopy(_, field_type) => {
            let field_type = utils::convert_to_zerocopy_type(field_type);
            quote! { <#field_type as ::light_zero_copy::traits::ZeroCopyNew<'static>>::ZeroCopyConfig }
        }
    };
    Ok(result)
}

/// Generate a configuration struct for a given struct
/// Returns None if no configuration is needed (no dynamic fields)
pub fn generate_config_struct(
    struct_name: &Ident,
    field_types: &[FieldType],
) -> syn::Result<Option<TokenStream2>> {
    let config_name = quote::format_ident!("{}Config", struct_name);

    // Generate config fields only for fields that require configuration
    let field_strategies: Vec<_> = field_types.iter().map(analyze_field_strategy).collect();

    let config_fields: Result<Vec<TokenStream2>, syn::Error> = field_types
        .iter()
        .zip(&field_strategies)
        .filter(|(_, strategy)| !matches!(strategy, FieldStrategy::FixedSize))
        .map(|(field_type, _)| -> syn::Result<TokenStream2> {
            let field_name = field_type.name();
            let config_type = config_type(field_type)?;
            Ok(quote! {
                pub #field_name: #config_type,
            })
        })
        .collect();
    let config_fields = config_fields?;

    let result = if config_fields.is_empty() {
        // If no fields require configuration, don't generate any config struct
        None
    } else {
        Some(quote! {
            #[derive(Debug, Clone, PartialEq)]
            pub struct #config_name {
                #(#config_fields)*
            }
        })
    };
    Ok(result)
}

/// Generate initialization logic for a field, with support for unit configs
pub fn generate_field_initialization_for_config(
    field_type: &FieldType,
) -> syn::Result<TokenStream2> {
    let result = match field_type {
        FieldType::VecU8(field_name) => {
            quote! {
                // Initialize the length prefix but don't use the returned ZeroCopySliceMut
                {
                    ::light_zero_copy::slice_mut::ZeroCopySliceMutBorsh::<u8>::new_at(
                        config.#field_name.into(),
                        __remaining_bytes
                    )?;
                }
                // Split off the length prefix (4 bytes) and get the slice
                if __remaining_bytes.len() < 4 {
                    return Err(::light_zero_copy::errors::ZeroCopyError::InsufficientMemoryAllocated(
                        __remaining_bytes.len(),
                        4
                    ));
                }
                let (_, __remaining_bytes) = __remaining_bytes.split_at_mut(4);
                let slice_len = match ::light_zero_copy::u32_to_usize(config.#field_name) {
                    Ok(len) => len,
                    Err(e) => return Err(e),
                };
                if __remaining_bytes.len() < slice_len {
                    return Err(::light_zero_copy::errors::ZeroCopyError::InsufficientMemoryAllocated(
                        __remaining_bytes.len(),
                        slice_len
                    ));
                }
                let (#field_name, __remaining_bytes) = __remaining_bytes.split_at_mut(slice_len);
            }
        }

        FieldType::VecCopy(field_name, inner_type) => {
            // Arrays are Copy types that don't implement ZeroCopyStructInnerMut.
            // They are used directly after type conversion (e.g., [u32; N] → [U32; N])
            let zerocopy_type = crate::shared::utils::convert_to_zerocopy_type(inner_type);
            quote! {
                let (#field_name, __remaining_bytes) = ::light_zero_copy::slice_mut::ZeroCopySliceMutBorsh::<#zerocopy_type>::new_at(
                    config.#field_name.into(),
                    __remaining_bytes
                )?;
            }
        }

        FieldType::VecDynamicZeroCopy(field_name, vec_type)
        | FieldType::DynamicZeroCopy(field_name, vec_type)
        | FieldType::Option(field_name, vec_type) => {
            quote! {
                let (#field_name, __remaining_bytes) = <#vec_type as ::light_zero_copy::traits::ZeroCopyNew<'a>>::new_zero_copy(
                    __remaining_bytes,
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
                let (#field_name, __remaining_bytes) = <#option_type as ::light_zero_copy::traits::ZeroCopyNew>::new_zero_copy(
                    __remaining_bytes,
                    (config.#field_name, ())
                )?;
            }
        }

        FieldType::OptionArray(field_name, array_type) => {
            let array_type_zerocopy = utils::convert_to_zerocopy_type(array_type);
            quote! {
                let (#field_name, __remaining_bytes) = <Option<#array_type_zerocopy> as ::light_zero_copy::traits::ZeroCopyNew>::new_zero_copy(
                    __remaining_bytes,
                    config.#field_name
                )?;
            }
        }

        // Fixed-size types that are struct fields (not meta fields) need initialization with () config
        FieldType::Primitive(field_name, field_type) => {
            quote! {
                let (#field_name, __remaining_bytes) = <#field_type as ::light_zero_copy::traits::ZeroCopyAtMut>::zero_copy_at_mut(__remaining_bytes)?;
            }
        }

        // Array fields that are struct fields (come after Vec/Option)
        FieldType::Array(field_name, array_type) => {
            let array_type_zerocopy = utils::convert_to_zerocopy_type(array_type);
            quote! {
                let (#field_name, __remaining_bytes) = ::light_zero_copy::Ref::<
                    &'a mut [u8],
                    #array_type_zerocopy
                >::from_prefix(__remaining_bytes)?;
            }
        }

        FieldType::Pubkey(field_name) => {
            quote! {
                let (#field_name, __remaining_bytes) = ::light_zero_copy::Ref::<
                    &'a mut [u8],
                    Pubkey
                >::from_prefix(__remaining_bytes)?;
            }
        }

        FieldType::Copy(field_name, field_type) => {
            quote! {
                let (#field_name, __remaining_bytes) = <#field_type as ::light_zero_copy::traits::ZeroCopyNew>::new_zero_copy(__remaining_bytes)?;
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
                {
                    let len = match ::light_zero_copy::u32_to_usize(config.#field_name) {
                        Ok(l) => l,
                        Err(e) => return Err(e),
                    };
                    let element_bytes = len;
                    match 4usize.checked_add(element_bytes) {
                        Some(total) => Ok(total),
                        None => Err(light_zero_copy::errors::ZeroCopyError::Size),
                    }
                }
            }
        }

        FieldType::VecCopy(field_name, inner_type) => {
            quote! {
                {
                    let len = match ::light_zero_copy::u32_to_usize(config.#field_name) {
                        Ok(l) => l,
                        Err(e) => return Err(e),
                    };
                    let elem_size = ::core::mem::size_of::<#inner_type>();
                    let element_bytes = match len.checked_mul(elem_size) {
                        Some(__remaining_bytes) => __remaining_bytes,
                        None => return Err(::light_zero_copy::errors::ZeroCopyError::Size),
                    };
                    match 4usize.checked_add(element_bytes) {
                        Some(total) => Ok(total),
                        None => Err(light_zero_copy::errors::ZeroCopyError::Size),
                    }
                }
            }
        }

        FieldType::VecDynamicZeroCopy(field_name, vec_type) => {
            quote! {
                <#vec_type as light_zero_copy::traits::ZeroCopyNew<'static>>::byte_len(&config.#field_name)
            }
        }

        // Option types
        FieldType::Option(field_name, option_type) => {
            quote! {
                <#option_type as light_zero_copy::traits::ZeroCopyNew<'static>>::byte_len(&config.#field_name)
            }
        }

        FieldType::OptionU64(field_name) => {
            quote! {
                <Option<u64> as light_zero_copy::traits::ZeroCopyNew<'static>>::byte_len(&(config.#field_name, ()))
            }
        }

        FieldType::OptionU32(field_name) => {
            quote! {
                <Option<u32> as light_zero_copy::traits::ZeroCopyNew<'static>>::byte_len(&(config.#field_name, ()))
            }
        }

        FieldType::OptionU16(field_name) => {
            quote! {
                <Option<u16> as light_zero_copy::traits::ZeroCopyNew<'static>>::byte_len(&(config.#field_name, ()))
            }
        }

        FieldType::OptionArray(field_name, array_type) => {
            let array_type_zerocopy = utils::convert_to_zerocopy_type(array_type);
            quote! {
                <Option<#array_type_zerocopy> as light_zero_copy::traits::ZeroCopyNew<'static>>::byte_len(&config.#field_name)
            }
        }

        // Fixed-size types don't need configuration and have known sizes
        FieldType::Primitive(_, field_type) => {
            let zerocopy_type = utils::convert_to_zerocopy_type(field_type);
            quote! {
                Ok(::core::mem::size_of::<#zerocopy_type>())
            }
        }

        FieldType::Array(_, array_type) => {
            quote! {
                Ok(::core::mem::size_of::<#array_type>())
            }
        }

        FieldType::Pubkey(_) => {
            quote! {
                Ok(32)  // Pubkey is always 32 bytes
            }
        }

        // Meta field types (should not appear in struct fields, but handle gracefully)
        FieldType::Copy(_, field_type) => {
            quote! {
                Ok(core::mem::size_of::<#field_type>())
            }
        }

        FieldType::DynamicZeroCopy(field_name, field_type) => {
            quote! {
                <#field_type as light_zero_copy::traits::ZeroCopyNew<'static>>::byte_len(&config.#field_name)
            }
        }
    };
    Ok(result)
}
