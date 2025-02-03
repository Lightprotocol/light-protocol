use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Type};

#[proc_macro_derive(ZeroCopyAccount)]
pub fn derive_zero_copy_account(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let z_struct_name = format_ident!("Z{}", struct_name);

    let fields = if let Data::Struct(data_struct) = input.data {
        data_struct.fields
    } else {
        panic!("ZeroCopyAccount can only be derived for structs");
    };

    let mut meta_fields = Vec::new();
    let mut optional_fields = Vec::new();
    let mut meta_flags = Vec::new();
    let mut from_conversions = Vec::new();
    let mut deserialize_code = Vec::new();
    let mut meta_struct_fields = Vec::new();

    for field in fields.iter() {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;

        if let Type::Path(type_path) = field_type {
            if let Some(segment) = type_path.path.segments.last() {
                if segment.ident == "Option" {
                    if let syn::Type::Path(inner_type) = parse_option_type(field_type) {
                        let inner_type_name = &inner_type.path.segments.last().unwrap().ident;
                        let inner_type_str = inner_type_name.to_string();

                        let flag_name = format_ident!("{}_option", field_name);
                        meta_flags.push(flag_name.clone());
                        meta_struct_fields.push(quote! {
                            pub #flag_name: u8
                        });

                        if is_fixed_size_type(&inner_type_str) {
                            optional_fields.push(quote! {
                                pub #field_name: Option<zerocopy::Ref<&'a [u8], #inner_type_name>>
                            });

                            deserialize_code.push(quote! {
                                let (#field_name, bytes) = if meta.#flag_name == 1 {
                                    let (field_val, bytes) = zerocopy::Ref::<&[u8], #inner_type_name>::zero_copy_at(bytes)?;
                                    (Some(field_val), bytes)
                                } else {
                                    (None, bytes)
                                };
                            });

                            from_conversions.push(quote! {
                                #field_name: compressed_account.#field_name.map(|x| *x),
                            });
                        } else {
                            let z_inner_type = format_ident!("Z{}", inner_type_name);
                            optional_fields.push(quote! {
                                pub #field_name: Option<#z_inner_type<'a>>
                            });

                            deserialize_code.push(quote! {
                                let (#field_name, bytes) = Option::<#z_inner_type>::zero_copy_at(bytes)?;
                            });

                            from_conversions.push(quote! {
                                #field_name: compressed_account.#field_name.as_ref().map(|data| data.into()),
                            });
                        }
                    }
                    continue;
                }
            }
        }

        let converted_type = convert_type(field_type);
        meta_struct_fields.push(quote! {
            pub #field_name: #converted_type
        });
        meta_fields.push(field_name.clone());
    }

    let meta_struct_name = format_ident!("{}DesMeta", struct_name);
    let meta_struct = quote! {
        #[repr(C)]
        #[derive(zerocopy::FromBytes, zerocopy::IntoBytes, zerocopy::Unaligned)]
        pub struct #meta_struct_name {
            #(#meta_struct_fields),*
        }
    };

    let z_struct = quote! {
        #[derive(Debug, PartialEq, Clone)]
        pub struct #z_struct_name<'a> {
            meta: zerocopy::Ref<&'a [u8], #meta_struct_name>,
            #(#optional_fields),*
        }
    };

    let deref_impl = quote! {
        impl std::ops::Deref for #z_struct_name<'_> {
            type Target = #meta_struct_name;

            fn deref(&self) -> &Self::Target {
                &self.meta
            }
        }
    };

    let from_impl = quote! {
        impl From<&#z_struct_name<'_>> for #struct_name {
            fn from(compressed_account: &#z_struct_name) -> Self {
                #struct_name {
                    #(#meta_fields: compressed_account.meta.#meta_fields.into()),*,
                    #(#from_conversions)*
                }
            }
        }
    };

    let deserialize_impl = quote! {
        impl<'a> light_zero_copy::borsh::Deserialize<'a> for #z_struct_name<'a> {
            type Output = Self;

            #[inline]
            fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), light_zero_copy::errors::ZeroCopyError> {
                let (meta, bytes) = zerocopy::Ref::<&[u8], #meta_struct_name>::from_prefix(bytes)?;
                #(#deserialize_code)*
                Ok((
                    #z_struct_name {
                        meta,
                        #(#optional_fields: #meta_flags),*
                    },
                    bytes
                ))
            }
        }
    };

    let expanded = quote! {
        #meta_struct
        #z_struct
        #deref_impl
        #from_impl
        #deserialize_impl
    };

    TokenStream::from(expanded)
}

fn parse_option_type(ty: &Type) -> &Type {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                        return inner_type;
                    }
                }
            }
        }
    }
    panic!("Expected Option type");
}

fn is_fixed_size_type(type_str: &str) -> bool {
    matches!(
        type_str,
        "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "[u8; 32]"
    )
}

fn convert_type(ty: &Type) -> proc_macro2::TokenStream {
    if let Type::Path(type_path) = ty {
        let type_ident = type_path.path.segments.last().unwrap().ident.to_string();
        match type_ident.as_str() {
            "u64" => quote! { zerocopy::U64 },
            "u32" => quote! { zerocopy::U32 },
            "u16" => quote! { zerocopy::U16 },
            _ => quote! { #ty },
        }
    } else {
        quote! { #ty }
    }
}