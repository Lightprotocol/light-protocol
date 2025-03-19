use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Field, Ident};

use crate::z_struct::{analyze_struct_fields, FieldType};

/// Generates meta field comparisons for PartialEq implementation
pub fn generate_meta_field_comparisons<'a>(
    meta_fields: &'a [&'a Field],
) -> impl Iterator<Item = TokenStream> + 'a {
    let field_types = analyze_struct_fields(meta_fields);

    field_types.into_iter().map(|field_type| match field_type {
        FieldType::IntegerU64(field_name) => {
            quote! {
                if other.#field_name != u64::from(meta.#field_name) as u64 {
                    return false;
                }
            }
        }
        FieldType::IntegerU32(field_name) => {
            quote! {
                if other.#field_name != u64::from(meta.#field_name) as u32 {
                    return false;
                }
            }
        }
        FieldType::IntegerU16(field_name) => {
            quote! {
                if other.#field_name != u64::from(meta.#field_name) as u16 {
                    return false;
                }
            }
        }
        FieldType::IntegerU8(field_name) => {
            quote! {
                if other.#field_name != u64::from(meta.#field_name) as u8 {
                    return false;
                }
            }
        }
        FieldType::Bool(field_name) => {
            quote! {
                if other.#field_name != (meta.#field_name > 0) {
                    return false;
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
    })
}

/// Generates struct field comparisons for PartialEq implementation
pub fn generate_struct_field_comparisons<'a>(
    struct_fields: &'a [&'a Field],
) -> impl Iterator<Item = TokenStream> + 'a {
    let field_types = analyze_struct_fields(struct_fields);
    if field_types.iter().any(|x| {
        if let FieldType::Option(_, _) = x {
            true
        } else {
            false
        }
    }) {
        unimplemented!("Options are not supported in ZeroCopyEq");
    }

    field_types.into_iter().map(|field_type| {
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
            FieldType::VecNonCopy(field_name, _) => {
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
                if field_type.to_token_stream().to_string() == "u8" {
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
            FieldType::IntegerU64(field_name) => {
                quote! {
                    if u64::from(*self.#field_name) != other.#field_name {
                        return false;
                    }
                }
            }
            FieldType::IntegerU32(field_name) => {
                quote! {
                    if u32::from(*self.#field_name) != other.#field_name {
                        return false;
                    }
                }
            }
            FieldType::IntegerU16(field_name) => {
                quote! {
                    if u16::from(*self.#field_name) != other.#field_name {
                        return false;
                    }
                }
            }
            FieldType::IntegerU8(field_name) => {
                quote! {
                    if self.#field_name != other.#field_name {
                        return false;
                    }
                }
            }
            FieldType::Bool(field_name) => {
                quote! {
                    if self.#field_name() != other.#field_name {
                        return false;
                    }
                }
            }
            FieldType::CopyU8Bool(field_name)
            | FieldType::Copy(field_name, _)
            | FieldType::NonCopy(field_name, _) => {
                quote! {
                    if self.#field_name != other.#field_name {
                        return false;
                    }
                }
            }
        }
    })
}

/// Generates the PartialEq implementation as a TokenStream
pub fn generate_partial_eq_impl(
    name: &Ident,
    z_struct_name: &Ident,
    z_struct_meta_name: &Ident,
    meta_fields: &[&Field],
    struct_fields: &[&Field],
) -> TokenStream {
    let struct_field_comparisons = generate_struct_field_comparisons(struct_fields);
    if !meta_fields.is_empty() {
        let meta_field_comparisons = generate_meta_field_comparisons(meta_fields);
        quote! {
            impl<'a> PartialEq<#name> for #z_struct_name<'a> {
                fn eq(&self, other: &#name) -> bool {
                    let meta: &#z_struct_meta_name = &self.__meta;
                    #(#meta_field_comparisons)*
                    #(#struct_field_comparisons)*
                    true
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

        }
    }
}
