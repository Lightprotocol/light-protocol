use proc_macro::TokenStream as ProcTokenStream;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_quote, DeriveInput, Field, Ident};

use crate::shared::{
    meta_struct, utils,
    z_enum::{generate_enum_deserialize_impl, generate_enum_zero_copy_struct_inner, generate_z_enum},
    z_struct::{analyze_struct_fields, generate_z_struct, FieldType},
};

/// Helper function to generate deserialize call pattern for a given type
fn generate_deserialize_call<const MUT: bool>(
    field_name: &syn::Ident,
    field_type: &syn::Type,
) -> TokenStream {
    let field_type = utils::convert_to_zerocopy_type(field_type);
    let trait_path = if MUT {
        quote!( as light_zero_copy::borsh_mut::DeserializeMut>::zero_copy_at_mut)
    } else {
        quote!( as light_zero_copy::borsh::Deserialize>::zero_copy_at)
    };

    quote! {
        let (#field_name, bytes) = <#field_type #trait_path(bytes)?;
    }
}

/// Generates field deserialization code for the Deserialize implementation
/// The `MUT` parameter controls whether to generate code for mutable or immutable references
pub fn generate_deserialize_fields<'a, const MUT: bool>(
    struct_fields: &'a [&'a Field],
) -> syn::Result<impl Iterator<Item = TokenStream> + 'a> {
    let field_types = analyze_struct_fields(struct_fields)?;

    let iterator = field_types.into_iter().map(move |field_type| {
        let mutability_tokens = if MUT {
            quote!(&'a mut [u8])
        } else {
            quote!(&'a [u8])
        };
        match field_type {
            FieldType::VecU8(field_name) => {
                if MUT {
                    quote! {
                        let (#field_name, bytes) = light_zero_copy::borsh_mut::borsh_vec_u8_as_slice_mut(bytes)?;
                    }
                } else {
                    quote! {
                        let (#field_name, bytes) = light_zero_copy::borsh::borsh_vec_u8_as_slice(bytes)?;
                    }
                }
            },
            FieldType::VecCopy(field_name, inner_type) => {
                let inner_type = utils::convert_to_zerocopy_type(inner_type);

                let trait_path = if MUT {
                    quote!(light_zero_copy::slice_mut::ZeroCopySliceMutBorsh::<'a, <#inner_type as light_zero_copy::borsh_mut::ZeroCopyStructInnerMut>::ZeroCopyInnerMut>)
                } else {
                    quote!(light_zero_copy::slice::ZeroCopySliceBorsh::<'a, <#inner_type as light_zero_copy::borsh::ZeroCopyStructInner>::ZeroCopyInner>)
                };
                quote! {
                    let (#field_name, bytes) = #trait_path::from_bytes_at(bytes)?;
                }
            },
            FieldType::VecDynamicZeroCopy(field_name, field_type) => {
                generate_deserialize_call::<MUT>(field_name, field_type)
            },
            FieldType::Array(field_name, field_type) => {
                let field_type = utils::convert_to_zerocopy_type(field_type);
                quote! {
                    let (#field_name, bytes) = light_zero_copy::Ref::<#mutability_tokens, #field_type>::from_prefix(bytes)?;
                }
            },
            FieldType::Option(field_name, field_type) => {
                generate_deserialize_call::<MUT>(field_name, field_type)
            },
            FieldType::Pubkey(field_name) => {
                generate_deserialize_call::<MUT>(field_name, &parse_quote!(Pubkey))
            },
            FieldType::Primitive(field_name, field_type) => {
                if MUT {
                    quote! {
                        let (#field_name, bytes) = <#field_type as light_zero_copy::borsh_mut::DeserializeMut>::zero_copy_at_mut(bytes)?;
                    }
                } else {
                    quote! {
                        let (#field_name, bytes) = <#field_type as light_zero_copy::borsh::Deserialize>::zero_copy_at(bytes)?;
                    }
                }
            },
            FieldType::Copy(field_name, field_type) => {
                let field_ty_zerocopy = utils::convert_to_zerocopy_type(field_type);
                quote! {
                    let (#field_name, bytes) = light_zero_copy::Ref::<#mutability_tokens, #field_ty_zerocopy>::from_prefix(bytes)?;
                }
            },
            FieldType::DynamicZeroCopy(field_name, field_type) => {
                generate_deserialize_call::<MUT>(field_name, field_type)
            },
            FieldType::OptionU64(field_name) => {
                let field_ty_zerocopy = utils::convert_to_zerocopy_type(&parse_quote!(u64));
                generate_deserialize_call::<MUT>(field_name, &parse_quote!(Option<#field_ty_zerocopy>))
            },
            FieldType::OptionU32(field_name) => {
                let field_ty_zerocopy = utils::convert_to_zerocopy_type(&parse_quote!(u32));
                generate_deserialize_call::<MUT>(field_name, &parse_quote!(Option<#field_ty_zerocopy>))
            },
            FieldType::OptionU16(field_name) => {
                let field_ty_zerocopy = utils::convert_to_zerocopy_type(&parse_quote!(u16));
                generate_deserialize_call::<MUT>(field_name, &parse_quote!(Option<#field_ty_zerocopy>))
            }
        }
    });
    Ok(iterator)
}

/// Generates field initialization code for the Deserialize implementation
pub fn generate_init_fields<'a>(
    struct_fields: &'a [&'a Field],
) -> impl Iterator<Item = TokenStream> + 'a {
    struct_fields.iter().map(|field| {
        let field_name = &field.ident;
        quote! { #field_name }
    })
}

/// Generates the Deserialize implementation as a TokenStream
/// The `MUT` parameter controls whether to generate code for mutable or immutable references
pub fn generate_deserialize_impl<const MUT: bool>(
    name: &Ident,
    z_struct_name: &Ident,
    z_struct_meta_name: &Ident,
    struct_fields: &[&Field],
    meta_is_empty: bool,
    byte_len_impl: TokenStream,
) -> syn::Result<TokenStream> {
    let z_struct_name = if MUT {
        format_ident!("{}Mut", z_struct_name)
    } else {
        z_struct_name.clone()
    };
    let z_struct_meta_name = if MUT {
        format_ident!("{}Mut", z_struct_meta_name)
    } else {
        z_struct_meta_name.clone()
    };

    // Define trait and types based on mutability
    let (trait_name, mutability, method_name) = if MUT {
        (
            quote!(light_zero_copy::borsh_mut::DeserializeMut),
            quote!(mut),
            quote!(zero_copy_at_mut),
        )
    } else {
        (
            quote!(light_zero_copy::borsh::Deserialize),
            quote!(),
            quote!(zero_copy_at),
        )
    };
    let (meta_des, meta) = if meta_is_empty {
        (quote!(), quote!())
    } else {
        (
            quote! {
                let (__meta, bytes) = light_zero_copy::Ref::< &'a #mutability [u8], #z_struct_meta_name>::from_prefix(bytes)?;
            },
            quote!(__meta,),
        )
    };
    let deserialize_fields = generate_deserialize_fields::<MUT>(struct_fields)?;
    let init_fields = generate_init_fields(struct_fields);

    let result = quote! {
        impl<'a> #trait_name<'a> for #name {
            type Output = #z_struct_name<'a>;

            fn #method_name(bytes: &'a #mutability [u8]) -> Result<(Self::Output, &'a #mutability [u8]), light_zero_copy::errors::ZeroCopyError> {
                #meta_des
                #(#deserialize_fields)*
                Ok((
                    #z_struct_name {
                        #meta
                        #(#init_fields,)*
                    },
                    bytes
                ))
            }

            #byte_len_impl
        }
    };
    Ok(result)
}

/// Generates the ZeroCopyStructInner implementation as a TokenStream
pub fn generate_zero_copy_struct_inner<const MUT: bool>(
    name: &Ident,
    z_struct_name: &Ident,
) -> syn::Result<TokenStream> {
    let result = if MUT {
        quote! {
            // ZeroCopyStructInner implementation
            impl light_zero_copy::borsh_mut::ZeroCopyStructInnerMut for #name {
                type ZeroCopyInnerMut = #z_struct_name<'static>;
            }
        }
    } else {
        quote! {
            // ZeroCopyStructInner implementation
            impl light_zero_copy::borsh::ZeroCopyStructInner for #name {
                type ZeroCopyInner = #z_struct_name<'static>;
            }
        }
    };
    Ok(result)
}

pub fn derive_zero_copy_impl(input: ProcTokenStream) -> syn::Result<proc_macro2::TokenStream> {
    // Parse the input DeriveInput
    let input: DeriveInput = syn::parse(input)?;

    let hasher = utils::struct_has_light_hasher_attribute(&input.attrs);

    // Disable light_hasher attribute due to Vec<u8>/&[u8] hash inconsistency
    if hasher {
        return Err(syn::Error::new_spanned(
            &input,
            "#[light_hasher] attribute is currently disabled due to hash inconsistency between Vec<u8> and &[u8] slice representations in ZStruct vs original struct. The original struct hashes Vec<u8> fields while the ZStruct hashes &[u8] slice fields, producing different hash values.",
        ));
    }

    // Process the input to extract information for both structs and enums
    let (name, z_name, input_type) = utils::process_input_generic(&input)?;

    match input_type {
        utils::InputType::Struct(fields) => {
            // Handle struct case (existing logic)
            let z_struct_name = z_name;
            let z_struct_meta_name = format_ident!("Z{}Meta", name);

            // Process the fields to separate meta fields and struct fields
            let (meta_fields, struct_fields) = utils::process_fields(fields);

            let meta_struct_def = if !meta_fields.is_empty() {
                meta_struct::generate_meta_struct::<false>(&z_struct_meta_name, &meta_fields, hasher)?
            } else {
                quote! {}
            };

            let z_struct_def = generate_z_struct::<false>(
                &z_struct_name,
                &z_struct_meta_name,
                &struct_fields,
                &meta_fields,
                hasher,
            )?;

            let zero_copy_struct_inner_impl =
                generate_zero_copy_struct_inner::<false>(name, &z_struct_name)?;

            let deserialize_impl = generate_deserialize_impl::<false>(
                name,
                &z_struct_name,
                &z_struct_meta_name,
                &struct_fields,
                meta_fields.is_empty(),
                quote! {},
            )?;

            // Combine all implementations
            Ok(quote! {
                #meta_struct_def
                #z_struct_def
                #zero_copy_struct_inner_impl
                #deserialize_impl
            })
        }
        utils::InputType::Enum(enum_data) => {
            // Handle enum case (new logic)
            let z_enum_name = z_name;

            let z_enum_def = generate_z_enum(&z_enum_name, enum_data)?;
            let deserialize_impl = generate_enum_deserialize_impl(name, &z_enum_name, enum_data)?;
            let zero_copy_struct_inner_impl = generate_enum_zero_copy_struct_inner(name, &z_enum_name)?;

            // Combine all implementations
            Ok(quote! {
                #z_enum_def
                #deserialize_impl
                #zero_copy_struct_inner_impl
            })
        }
    }
}
