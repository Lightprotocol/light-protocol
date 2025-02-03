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

    let mut meta_struct_fields = Vec::new();
    let mut optional_fields = Vec::new();
    let mut deserialize_code = Vec::new();
    let mut deserialize_field_inits = Vec::new();
    let mut from_field_inits = Vec::new();

    for field in fields.iter() {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;

        if let Type::Path(type_path) = field_type {
            if let Some(segment) = type_path.path.segments.last() {
                if segment.ident == "Option" {
                    if let Type::Path(inner_type) = parse_option_type(field_type) {
                        let inner_type_name = &inner_type.path.segments.last().unwrap().ident;
                        let flag_name = format_ident!("{}_option", field_name);

                        meta_struct_fields.push(quote! {
                            pub #flag_name: u8
                        });

                        if is_fixed_size_type(&inner_type_name.to_string()) {
                            optional_fields.push(quote! {
                                pub #field_name: Option<zerocopy::Ref<&'a [u8], #inner_type_name>>
                            });

                            deserialize_code.push(quote! {
                                let (#field_name, bytes) = if meta.#flag_name == 1 {
                                    let val = zerocopy::Ref::new(&bytes[..std::mem::size_of::<#inner_type_name>()])
                                        .ok_or(light_zero_copy::errors::ZeroCopyError::InvalidData)?;
                                    (Some(val), &bytes[std::mem::size_of::<#inner_type_name>()..])
                                } else {
                                    (None, bytes)
                                };
                            });

                            from_field_inits.push(quote! {
                                #field_name: z_account.#field_name.map(|x| *x)
                            });

                            deserialize_field_inits.push(quote! {
                                #field_name: #field_name
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
        from_field_inits.push(quote! {
            #field_name: z_account.meta.#field_name.get()
        });
    }

    let meta_struct_name = format_ident!("{}DesMeta", struct_name);

    let expanded = quote! {
        #[repr(C)]
        #[derive(zerocopy::FromBytes, zerocopy::AsBytes, zerocopy::Unaligned, Debug, Clone, PartialEq)]
        pub struct #meta_struct_name {
            #(#meta_struct_fields),*
        }

        #[derive(Debug, PartialEq)]
        pub struct #z_struct_name<'a> {
            meta: zerocopy::Ref<&'a [u8], #meta_struct_name>,
            #(#optional_fields),*
        }

        impl std::ops::Deref for #z_struct_name<'_> {
            type Target = #meta_struct_name;
            fn deref(&self) -> &Self::Target {
                &self.meta
            }
        }

        impl From<&#z_struct_name<'_>> for #struct_name {
            fn from(z_account: &#z_struct_name) -> Self {
                Self {
                    #(#from_field_inits),*
                }
            }
        }

        impl<'a> light_zero_copy::borsh::Deserialize<'a> for #z_struct_name<'a> {
            type Output = Self;

            fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), light_zero_copy::errors::ZeroCopyError> {
                let meta = zerocopy::Ref::new(bytes)
                    .ok_or(light_zero_copy::errors::ZeroCopyError::InvalidData)?;
                let mut bytes = &bytes[std::mem::size_of::<#meta_struct_name>()..];
                #(#deserialize_code)*
                Ok((
                    Self {
                        meta,
                        #(#deserialize_field_inits),*
                    },
                    bytes
                ))
            }
        }
    };

    TokenStream::from(expanded)
}

fn parse_option_type(ty: &Type) -> &Type {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                    return inner_type;
                }
            }
        }
    }
    panic!("Expected Option type")
}

fn is_fixed_size_type(type_str: &str) -> bool {
    matches!(type_str, "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "[u8; 32]")
}

fn convert_type(ty: &Type) -> proc_macro2::TokenStream {
    if let Type::Path(type_path) = ty {
        let type_str = type_path.path.segments.last().unwrap().ident.to_string();
        match type_str.as_str() {
            "u64" => quote! { zerocopy::U64 },
            "u32" => quote! { zerocopy::U32 },
            "u16" => quote! { zerocopy::U16 },
            _ => quote! { #ty },
        }
    } else {
        quote! { #ty }
    }
}