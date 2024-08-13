use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Fields, ItemStruct, Result};

pub(crate) fn hasher(input: ItemStruct) -> Result<TokenStream> {
    let struct_name = &input.ident;

    let (impl_gen, type_gen, where_clause) = input.generics.split_for_impl();

    let fields = match input.fields {
        Fields::Named(fields) => fields,
        _ => {
            return Err(Error::new_spanned(
                input,
                "Only structs with named fields are supported",
            ))
        }
    };

    let field_into_bytes_calls = fields
        .named
        .iter()
        .filter(|field| {
            !field.attrs.iter().any(|attr| {
                if let Some(attr_ident) = attr.path.get_ident() {
                    attr_ident == "skip"
                } else {
                    false
                }
            })
        })
        .map(|field| {
            let field_name = &field.ident;
            let truncate = field.attrs.iter().any(|attr| {
                if let Some(attr_ident) = attr.path.get_ident() {
                    attr_ident == "truncate"
                } else {
                    false
                }
            });
            if truncate {
                quote! {
                    let truncated_bytes = self
                        .#field_name
                        .as_byte_vec()
                        .iter()
                        .map(|bytes| {
                            let (bytes, _) = ::light_utils::hash_to_bn254_field_size_be(bytes).expect(
                                "Could not truncate the field #field_name to the BN254 prime field"
                            );
                            bytes.to_vec()
                        })
                        .collect::<Vec<Vec<u8>>>();
                    result.extend_from_slice(truncated_bytes.as_slice());
                }
            } else {
                quote! {
                    result.extend_from_slice(self.#field_name.as_byte_vec().as_slice());
                }
            }
        })
        .collect::<Vec<_>>();

    Ok(quote! {
        impl #impl_gen ::light_hasher::bytes::AsByteVec for #struct_name #type_gen #where_clause {
            fn as_byte_vec(&self) -> Vec<Vec<u8>> {
                use ::light_hasher::bytes::AsByteVec;

                let mut result: Vec<Vec<u8>> = Vec::new();
                #(#field_into_bytes_calls)*
                result
            }
        }

        impl #impl_gen ::light_hasher::DataHasher for #struct_name #type_gen #where_clause {
            fn hash<H: light_hasher::Hasher>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::errors::HasherError> {
                use ::light_hasher::bytes::AsByteVec;

                H::hashv(self.as_byte_vec().iter().map(|v| v.as_slice()).collect::<Vec<_>>().as_slice())
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use syn::parse_quote;

    #[test]
    fn test_light_hasher() {
        let input: ItemStruct = parse_quote! {
            struct MyAccount {
                a: u32,
                b: i32,
                c: u64,
                d: i64,
            }
        };

        let output = hasher(input).unwrap();
        let output = output.to_string();

        println!("{output}");
    }
}
