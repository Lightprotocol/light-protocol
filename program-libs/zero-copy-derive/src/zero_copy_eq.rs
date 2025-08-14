use proc_macro::TokenStream as ProcTokenStream;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Field, Ident};

use crate::shared::{
    from_impl, utils,
    z_struct::{analyze_struct_fields, FieldType},
};

/// Helper function to generate Option field comparison with custom comparison expression
fn generate_option_comparison(
    field_name: &syn::Ident,
    comparison_expr: TokenStream,
) -> TokenStream {
    quote! {
        match (&self.#field_name, &other.#field_name) {
            (Some(z_ref), Some(other_val)) => {
                if #comparison_expr {
                    return false;
                }
            }
            (None, None) => {},
            _ => return false,
        }
    }
}

/// Generates meta field comparisons for PartialEq implementation
pub fn generate_meta_field_comparisons<'a>(
    meta_fields: &'a [&'a Field],
) -> syn::Result<impl Iterator<Item = TokenStream> + 'a> {
    let field_types = analyze_struct_fields(meta_fields)?;

    let iterator = field_types.into_iter().map(|field_type| match field_type {
        FieldType::Primitive(field_name, field_type) => {
            match () {
                _ if utils::is_specific_primitive_type(field_type, "u8") => quote! {
                    if other.#field_name != meta.#field_name {
                        return false;
                    }
                },
                _ if utils::is_specific_primitive_type(field_type, "bool") => quote! {
                    if other.#field_name != (meta.#field_name > 0) {
                        return false;
                    }
                },
                _ => {
                    // For u64, u32, u16 - use the type's from() method
                    quote! {
                        if other.#field_name != #field_type::from(meta.#field_name) {
                            return false;
                        }
                    }
                }
            }
        }
        _ => {
            let field_name = field_type.name();
            quote! {
                if other.#field_name != meta.#field_name {
                    return false;
                }
            }
        }
    });
    Ok(iterator)
}

/// Generates struct field comparisons for PartialEq implementation
pub fn generate_struct_field_comparisons<'a, const MUT: bool>(
    struct_fields: &'a [&'a Field],
) -> syn::Result<impl Iterator<Item = TokenStream> + 'a> {
    let field_types = analyze_struct_fields(struct_fields)?;
    if field_types
        .iter()
        .any(|x| matches!(x, FieldType::Option(_, _)))
    {
        return Err(syn::Error::new_spanned(
            struct_fields[0],
            "Options are not supported in ZeroCopyEq",
        ));
    }

    let iterator = field_types.into_iter().map(|field_type| {
        match field_type {
            FieldType::VecU8(field_name) => {
                quote! {
                    if self.#field_name != other.#field_name.as_slice() {
                        return false;
                    }
                }
            }
            FieldType::VecCopy(field_name, _) => {
                quote! {
                    if self.#field_name.as_slice() != other.#field_name.as_slice() {
                        return false;
                    }
                }
            }
            FieldType::VecDynamicZeroCopy(field_name, _) => {
                quote! {
                    if self.#field_name.as_slice() != other.#field_name.as_slice() {
                        return false;
                    }
                }
            }
            FieldType::Array(field_name, _) => {
                quote! {
                    if *self.#field_name != other.#field_name {
                        return false;
                    }
                }
            }
            FieldType::Option(field_name, field_type) => {
                if utils::is_specific_primitive_type(field_type, "u8") {
                    quote! {
                        if self.#field_name.is_some() && other.#field_name.is_some() {
                            if self.#field_name.as_ref().unwrap() != other.#field_name.as_ref().unwrap() {
                                return false;
                            }
                        } else if self.#field_name.is_some() || other.#field_name.is_some() {
                            return false;
                        }
                    }
                }
                // TODO: handle issue that structs need * == *, arrays need ** == *
                // else if crate::utils::is_copy_type(field_type) {
                //     quote! {
                //         if self.#field_name.is_some() && other.#field_name.is_some() {
                //             if **self.#field_name.as_ref().unwrap() != *other.#field_name.as_ref().unwrap() {
                //                 return false;
                //             }
                //         } else if self.#field_name.is_some() || other.#field_name.is_some() {
                //             return false;
                //         }
                //     }
                // }
                else   {
                    quote! {
                        if self.#field_name.is_some() && other.#field_name.is_some() {
                            if **self.#field_name.as_ref().unwrap() != *other.#field_name.as_ref().unwrap() {
                                return false;
                            }
                        } else if self.#field_name.is_some() || other.#field_name.is_some() {
                            return false;
                        }
                    }
                }

            }
            FieldType::Pubkey(field_name) => {
                quote! {
                    if *self.#field_name != other.#field_name {
                        return false;
                    }
                }
            }
            FieldType::Primitive(field_name, field_type) => {
                match () {
                    _ if utils::is_specific_primitive_type(field_type, "u8") =>
                        if MUT {
                            quote! {
                                if *self.#field_name != other.#field_name {
                                    return false;
                                }
                            }
                        } else {
                            quote! {
                                if self.#field_name != other.#field_name {
                                    return false;
                                }
                            }
                        },
                    _ if utils::is_specific_primitive_type(field_type, "bool") =>
                        if MUT {
                            quote! {
                                if (*self.#field_name > 0) != other.#field_name {
                                    return false;
                                }
                            }
                        } else {
                            quote! {
                                if (self.#field_name > 0) != other.#field_name {
                                    return false;
                                }
                            }
                        },
                    _ => {
                        // For u64, u32, u16 - use the type's from() method
                        quote! {
                            if #field_type::from(*self.#field_name) != other.#field_name {
                                return false;
                            }
                        }
                    }
                }
            }
            FieldType::Copy(field_name, _)
            | FieldType::DynamicZeroCopy(field_name, _) => {
                quote! {
                    if self.#field_name != other.#field_name {
                        return false;
                    }
                }
            },
            FieldType::OptionU64(field_name) => {
                generate_option_comparison(field_name, quote! { u64::from(**z_ref) != *other_val })
            }
            FieldType::OptionU32(field_name) => {
                generate_option_comparison(field_name, quote! { u32::from(**z_ref) != *other_val })
            }
            FieldType::OptionU16(field_name) => {
                generate_option_comparison(field_name, quote! { u16::from(**z_ref) != *other_val })
            }
            FieldType::OptionArray(field_name, _) => {
                generate_option_comparison(field_name, quote! { **z_ref != *other_val })
            }
        }
    });
    Ok(iterator)
}

/// Generates the PartialEq implementation as a TokenStream
pub fn generate_partial_eq_impl<const MUT: bool>(
    name: &Ident,
    z_struct_name: &Ident,
    z_struct_meta_name: &Ident,
    meta_fields: &[&Field],
    struct_fields: &[&Field],
) -> syn::Result<TokenStream> {
    let struct_field_comparisons = generate_struct_field_comparisons::<MUT>(struct_fields)?;
    let result = if !meta_fields.is_empty() {
        let meta_field_comparisons = generate_meta_field_comparisons(meta_fields)?;
        quote! {
            impl<'a> PartialEq<#name> for #z_struct_name<'a> {
                fn eq(&self, other: &#name) -> bool {
                    let meta: &#z_struct_meta_name = &self.__meta;
                    #(#meta_field_comparisons)*
                    #(#struct_field_comparisons)*
                    true
                }
            }

            impl<'a> PartialEq<#z_struct_name<'a>> for #name {
                fn eq(&self, other: &#z_struct_name<'a>) -> bool {
                    other.eq(self)
                }
            }
        }
    } else {
        quote! {
            impl<'a> PartialEq<#name> for #z_struct_name<'a> {
                fn eq(&self, other: &#name) -> bool {
                    #(#struct_field_comparisons)*
                    true
                }
            }

            impl<'a> PartialEq<#z_struct_name<'a>> for #name {
                fn eq(&self, other: &#z_struct_name<'a>) -> bool {
                    other.eq(self)
                }
            }

        }
    };
    Ok(result)
}

pub fn derive_zero_copy_eq_impl(input: ProcTokenStream) -> syn::Result<proc_macro2::TokenStream> {
    // Parse the input DeriveInput
    let input: DeriveInput = syn::parse(input)?;

    // Validate that struct has #[repr(C)] attribute
    utils::validate_repr_c_required(&input.attrs, "ZeroCopyEq")?;

    // Process the input to extract struct information
    let (name, z_struct_name, z_struct_meta_name, fields) = utils::process_input(&input)?;

    // Process the fields to separate meta fields and struct fields
    let (meta_fields, struct_fields) = utils::process_fields(fields);

    // Generate the PartialEq implementation
    let partial_eq_impl = generate_partial_eq_impl::<false>(
        name,
        &z_struct_name,
        &z_struct_meta_name,
        &meta_fields,
        &struct_fields,
    )?;

    // Generate From implementations
    let from_impl =
        from_impl::generate_from_impl::<false>(name, &z_struct_name, &meta_fields, &struct_fields)?;

    Ok(quote! {
        #partial_eq_impl
        #from_impl
    })
}
