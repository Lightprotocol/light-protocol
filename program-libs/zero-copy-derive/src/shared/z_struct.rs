use proc_macro2::TokenStream;
use quote::{format_ident, quote, TokenStreamExt};
use syn::{parse_quote, Field, Ident, Type};

use super::utils;

/// Enum representing the different field types for zero-copy struct
/// (Name, Type)
#[derive(Debug)]
pub enum FieldType<'a> {
    VecU8(&'a Ident),
    VecCopy(&'a Ident, &'a Type),
    VecDynamicZeroCopy(&'a Ident, &'a Type),
    Array(&'a Ident, &'a Type),
    Option(&'a Ident, &'a Type),
    OptionU64(&'a Ident),
    OptionU32(&'a Ident),
    OptionU16(&'a Ident),
    OptionArray(&'a Ident, &'a Type),
    Pubkey(&'a Ident),
    Primitive(&'a Ident, &'a Type),
    Copy(&'a Ident, &'a Type),
    DynamicZeroCopy(&'a Ident, &'a Type),
}

impl<'a> FieldType<'a> {
    /// Get the name of the field
    pub fn name(&self) -> &'a Ident {
        match self {
            FieldType::VecU8(name) => name,
            FieldType::VecCopy(name, _) => name,
            FieldType::VecDynamicZeroCopy(name, _) => name,
            FieldType::Array(name, _) => name,
            FieldType::Option(name, _) => name,
            FieldType::OptionU64(name) => name,
            FieldType::OptionU32(name) => name,
            FieldType::OptionU16(name) => name,
            FieldType::OptionArray(name, _) => name,
            FieldType::Pubkey(name) => name,
            FieldType::Primitive(name, _) => name,
            FieldType::Copy(name, _) => name,
            FieldType::DynamicZeroCopy(name, _) => name,
        }
    }
}

/// Classify a Vec type based on its inner type
fn classify_vec_type<'a>(
    field_name: &'a Ident,
    field_type: &'a Type,
    inner_type: &'a Type,
) -> FieldType<'a> {
    if utils::is_specific_primitive_type(inner_type, "u8") {
        FieldType::VecU8(field_name)
    } else if utils::is_copy_type(inner_type) {
        FieldType::VecCopy(field_name, inner_type)
    } else {
        FieldType::VecDynamicZeroCopy(field_name, field_type)
    }
}

/// Classify an Option type based on its inner type
fn classify_option_type<'a>(
    field_name: &'a Ident,
    field_type: &'a Type,
    inner_type: &'a Type,
) -> FieldType<'a> {
    if utils::is_primitive_integer(inner_type) {
        match () {
            _ if utils::is_specific_primitive_type(inner_type, "u64") => {
                FieldType::OptionU64(field_name)
            }
            _ if utils::is_specific_primitive_type(inner_type, "u32") => {
                FieldType::OptionU32(field_name)
            }
            _ if utils::is_specific_primitive_type(inner_type, "u16") => {
                FieldType::OptionU16(field_name)
            }
            _ => FieldType::Option(field_name, field_type),
        }
    } else {
        FieldType::Option(field_name, field_type)
    }
}

/// Classify a primitive integer type
fn classify_integer_type<'a>(
    field_name: &'a Ident,
    field_type: &'a Type,
) -> syn::Result<FieldType<'a>> {
    match () {
        _ if utils::is_specific_primitive_type(field_type, "u64")
            | utils::is_specific_primitive_type(field_type, "u32")
            | utils::is_specific_primitive_type(field_type, "u16")
            | utils::is_specific_primitive_type(field_type, "u8")
            | utils::is_specific_primitive_type(field_type, "i64")
            | utils::is_specific_primitive_type(field_type, "i32")
            | utils::is_specific_primitive_type(field_type, "i16")
            | utils::is_specific_primitive_type(field_type, "i8") =>
        {
            Ok(FieldType::Primitive(field_name, field_type))
        }
        _ => Err(syn::Error::new_spanned(
            field_type,
            "Unsupported integer type. Only u8, u16, u32, u64, i8, i16, i32, and i64 are supported",
        )),
    }
}

/// Classify a Copy type
fn classify_copy_type<'a>(field_name: &'a Ident, field_type: &'a Type) -> FieldType<'a> {
    if utils::is_specific_primitive_type(field_type, "u8")
        || utils::is_specific_primitive_type(field_type, "bool")
    {
        FieldType::Primitive(field_name, field_type)
    } else {
        FieldType::Copy(field_name, field_type)
    }
}

/// Classify a single field into its FieldType
fn classify_field<'a>(field_name: &'a Ident, field_type: &'a Type) -> syn::Result<FieldType<'a>> {
    // Vec types
    if utils::is_vec_type(field_type) {
        return match utils::get_vec_inner_type(field_type) {
            Some(inner_type) => Ok(classify_vec_type(field_name, field_type, inner_type)),
            None => Err(syn::Error::new_spanned(
                field_type,
                "Could not determine inner type of Vec",
            )),
        };
    }

    // Array types
    if let Type::Array(_) = field_type {
        return Ok(FieldType::Array(field_name, field_type));
    }

    // Option types
    if utils::is_option_type(field_type) {
        return match utils::get_option_inner_type(field_type) {
            Some(inner_type) => {
                // Special handling for Option<[T; N]>
                if matches!(inner_type, Type::Array(_)) {
                    return Ok(FieldType::OptionArray(field_name, inner_type));
                }
                Ok(classify_option_type(field_name, field_type, inner_type))
            }
            None => Ok(FieldType::Option(field_name, field_type)),
        };
    }

    // Simple type dispatch
    match () {
        _ if utils::is_pubkey_type(field_type) => Ok(FieldType::Pubkey(field_name)),
        _ if utils::is_bool_type(field_type) => Ok(FieldType::Primitive(field_name, field_type)),
        _ if utils::is_primitive_integer(field_type) => {
            classify_integer_type(field_name, field_type)
        }
        _ if utils::is_copy_type(field_type) => Ok(classify_copy_type(field_name, field_type)),
        _ => Ok(FieldType::DynamicZeroCopy(field_name, field_type)),
    }
}

/// Analyze struct fields and return vector of FieldType enums
pub fn analyze_struct_fields<'a>(
    struct_fields: &'a [&'a Field],
) -> syn::Result<Vec<FieldType<'a>>> {
    struct_fields
        .iter()
        .map(|field| {
            let field_name = field
                .ident
                .as_ref()
                .ok_or_else(|| syn::Error::new_spanned(field, "Field must have a name"))?;
            classify_field(field_name, &field.ty)
        })
        .collect()
}

/// Generate struct fields with zerocopy types based on field type enum
fn generate_struct_fields_with_zerocopy_types<'a, const MUT: bool>(
    struct_fields: &'a [&'a Field],
    hasher: &'a bool,
) -> syn::Result<impl Iterator<Item = TokenStream> + 'a> {
    let field_types = analyze_struct_fields(struct_fields)?;
    let iterator = field_types
        .into_iter()
        .zip(struct_fields.iter())
        .map(|(field_type, field)| {
            let attributes = if *hasher {
                field
                    .attrs
                    .iter()
                    .map(|attr| {
                        quote! { #attr }
                    })
                    .collect::<Vec<_>>()
            } else {
                vec![quote! {}]
            };
            let (mutability, import_path, import_slice, camel_case_suffix): (
                syn::Type,
                syn::Ident,
                syn::Ident,
                String,
            ) = if MUT {
                (
                    parse_quote!(&'a mut [u8]),
                    format_ident!("traits"),
                    format_ident!("slice_mut"),
                    String::from("Mut"),
                )
            } else {
                (
                    parse_quote!(&'a [u8]),
                    format_ident!("traits"),
                    format_ident!("slice"),
                    String::new(),
                )
            };
            let deserialize_ident = if MUT {
                format_ident!("ZeroCopyAtMut")
            } else {
                format_ident!("ZeroCopyAt")
            };
            let associated_type_ident = if MUT {
                format_ident!("ZeroCopyAtMut")
            } else {
                format_ident!("ZeroCopyAt")
            };
            let trait_name: syn::Type = parse_quote!(light_zero_copy::#import_path::#deserialize_ident);
            let slice_ident = format_ident!("ZeroCopySlice{}Borsh", camel_case_suffix);
            let slice_name: syn::Type = parse_quote!(light_zero_copy::#import_slice::#slice_ident);
            let struct_inner_ident = format_ident!("ZeroCopyStructInner{}", camel_case_suffix);
            let inner_ident = format_ident!("ZeroCopyInner{}", camel_case_suffix);
            let struct_inner_trait_name: syn::Type = parse_quote!(light_zero_copy::#import_path::#struct_inner_ident::#inner_ident);
            match field_type {
                FieldType::VecU8(field_name) => {
                    quote! {
                        #(#attributes)*
                        pub #field_name: #mutability
                    }
                }
                FieldType::VecCopy(field_name, inner_type) => {
                    let zerocopy_type = utils::convert_to_zerocopy_type(inner_type);

                    if utils::needs_struct_inner_trait(inner_type) {
                        // Custom structs need to use the trait's associated type
                        quote! {
                            #(#attributes)*
                            pub #field_name: #slice_name<'a, <#zerocopy_type as #struct_inner_trait_name>::#associated_type_ident>
                        }
                    } else {
                        // Arrays and primitives can be used directly after type conversion
                        quote! {
                            #(#attributes)*
                            pub #field_name: #slice_name<'a, #zerocopy_type>
                        }
                    }
                }
                FieldType::VecDynamicZeroCopy(field_name, field_type) => {
                    let field_type = utils::convert_to_zerocopy_type(field_type);
                    quote! {
                        #(#attributes)*
                        pub #field_name: <#field_type as #trait_name<'a>>::#associated_type_ident
                    }
                }
                FieldType::Array(field_name, field_type) => {
                    let field_type = utils::convert_to_zerocopy_type(field_type);
                    quote! {
                        #(#attributes)*
                        pub #field_name: ::light_zero_copy::Ref<#mutability , #field_type>
                    }
                }
                FieldType::Option(field_name, field_type) => {
                    let field_type = utils::convert_to_zerocopy_type(field_type);
                    quote! {
                        #(#attributes)*
                        pub #field_name: <#field_type as #trait_name<'a>>::#associated_type_ident
                    }
                }
                FieldType::OptionU64(field_name) => {
                    let field_ty_zerocopy = utils::convert_to_zerocopy_type(&parse_quote!(u64));
                    quote! {
                        #(#attributes)*
                        pub #field_name: Option<::light_zero_copy::Ref<#mutability, #field_ty_zerocopy>>
                    }
                }
                FieldType::OptionU32(field_name) => {
                    let field_ty_zerocopy = utils::convert_to_zerocopy_type(&parse_quote!(u32));
                    quote! {
                        #(#attributes)*
                        pub #field_name: Option<::light_zero_copy::Ref<#mutability, #field_ty_zerocopy>>
                    }
                }
                FieldType::OptionU16(field_name) => {
                    let field_ty_zerocopy = utils::convert_to_zerocopy_type(&parse_quote!(u16));
                    quote! {
                        #(#attributes)*
                        pub #field_name: Option<::light_zero_copy::Ref<#mutability, #field_ty_zerocopy>>
                    }
                }
                FieldType::OptionArray(field_name, array_type) => {
                    let array_type_zerocopy = utils::convert_to_zerocopy_type(array_type);
                    quote! {
                        #(#attributes)*
                        pub #field_name: Option<::light_zero_copy::Ref<#mutability, #array_type_zerocopy>>
                    }
                }
                FieldType::Pubkey(field_name) => {
                    quote! {
                        #(#attributes)*
                        pub #field_name: <Pubkey as #trait_name<'a>>::#associated_type_ident
                    }
                }
                FieldType::Primitive(field_name, field_type) => {
                    quote! {
                        #(#attributes)*
                        pub #field_name: <#field_type as #trait_name<'a>>::#associated_type_ident
                    }
                }
                FieldType::Copy(field_name, field_type) => {
                    let zerocopy_type = utils::convert_to_zerocopy_type(field_type);
                    quote! {
                        #(#attributes)*
                        pub #field_name: ::light_zero_copy::Ref<#mutability , #zerocopy_type>
                    }
                }
                FieldType::DynamicZeroCopy(field_name, field_type) => {
                    quote! {
                        #(#attributes)*
                        pub #field_name: <#field_type as #trait_name<'a>>::#associated_type_ident
                    }
                }
            }
        });
    Ok(iterator)
}

/// Generate accessor methods for boolean fields in struct_fields.
/// We need accessors because booleans are stored as u8.
fn generate_bool_accessor_methods<'a, const MUT: bool>(
    struct_fields: &'a [&'a Field],
) -> impl Iterator<Item = TokenStream> + 'a {
    struct_fields.iter().filter_map(|field| {
        let field_name = &field.ident;
        let field_type = &field.ty;

        if utils::is_bool_type(field_type) {
            let comparison = if MUT {
                quote! { *self.#field_name > 0 }
            } else {
                quote! { self.#field_name > 0 }
            };

            Some(quote! {
                pub fn #field_name(&self) -> bool {
                    #comparison
                }
            })
        } else {
            None
        }
    })
}

/// Generates the ZStruct definition as a TokenStream
pub fn generate_z_struct<const MUT: bool>(
    z_struct_name: &Ident,
    z_struct_meta_name: &Ident,
    struct_fields: &[&Field],
    meta_fields: &[&Field],
    hasher: bool,
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
    let mutability: syn::Type = if MUT {
        parse_quote!(&'a mut [u8])
    } else {
        parse_quote!(&'a [u8])
    };

    let derive_clone = if MUT {
        quote! {}
    } else {
        quote! {, Clone }
    };
    let struct_fields_with_zerocopy_types: Vec<TokenStream> =
        generate_struct_fields_with_zerocopy_types::<MUT>(struct_fields, &hasher)?.collect();

    let derive_hasher = if hasher {
        quote! {
            , LightHasher
        }
    } else {
        quote! {}
    };
    let hasher_flatten = quote! {};

    let partial_eq_derive = if MUT { quote!() } else { quote!(, PartialEq) };

    let mut z_struct = if meta_fields.is_empty() {
        quote! {
            // ZStruct
            #[derive(Debug #partial_eq_derive #derive_clone #derive_hasher)]
            pub struct #z_struct_name<'a> {
                #(#struct_fields_with_zerocopy_types,)*
            }
        }
    } else {
        let mut tokens = quote! {
            // ZStruct
            #[derive(Debug #partial_eq_derive #derive_clone #derive_hasher)]
            pub struct #z_struct_name<'a> {
                #hasher_flatten
                __meta: ::light_zero_copy::Ref<#mutability, #z_struct_meta_name>,
                #(#struct_fields_with_zerocopy_types,)*
            }
            impl<'a> ::core::ops::Deref for #z_struct_name<'a> {
                type Target =  ::light_zero_copy::Ref<#mutability  , #z_struct_meta_name>;

                fn deref(&self) -> &Self::Target {
                    &self.__meta
                }
            }
        };

        if MUT {
            tokens.append_all(quote! {
                impl<'a> ::core::ops::DerefMut for #z_struct_name<'a> {
                    fn deref_mut(&mut self) ->  &mut Self::Target {
                        &mut self.__meta
                    }
                }
            });
        }
        tokens
    };

    // Only generate impl block if there are boolean fields that need accessors
    let has_bool_in_meta = meta_fields.iter().any(|f| utils::is_bool_type(&f.ty));
    if has_bool_in_meta {
        let meta_bool_accessor_methods = generate_bool_accessor_methods::<false>(meta_fields);
        z_struct.append_all(quote! {
            impl<'a> #z_struct_name<'a> {
                #(#meta_bool_accessor_methods)*
            }
        })
    }

    // Only generate impl block if there are boolean fields that need accessors
    let has_bool_in_struct = struct_fields.iter().any(|f| utils::is_bool_type(&f.ty));
    if has_bool_in_struct {
        let bool_accessor_methods = generate_bool_accessor_methods::<MUT>(struct_fields);
        z_struct.append_all(quote! {
            impl<'a> #z_struct_name<'a> {
                #(#bool_accessor_methods)*
            }
        });
    }
    Ok(z_struct)
}
