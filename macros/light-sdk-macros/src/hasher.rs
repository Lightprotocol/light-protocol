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
            !field.attrs.iter().any(|attr| attr.path().is_ident("skip"))
        })
        .map(|field| {
            let field_name = &field.ident;
            let truncate = field.attrs.iter().any(|attr| attr.path().is_ident("truncate"));
            let nested = field.attrs.iter().any(|attr| attr.path().is_ident("nested"));
            if nested {
                quote! {
                    result.extend(self.#field_name.as_byte_vec());
                }
            } else if truncate {
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
                    result.extend(truncated_bytes);
                }
            } else {
                quote! {
                    result.extend(self.#field_name.as_byte_vec());
                }
            }
        })
        .collect::<Vec<_>>();

    Ok(quote! {
        impl #impl_gen ::light_hasher::bytes::AsByteVec for #struct_name #type_gen #where_clause {
            fn as_byte_vec(&self) -> Vec<Vec<u8>> {
                use ::light_hasher::bytes::AsByteVec;

                let mut result: Vec<Vec<u8>> = Vec::new();
                #(
                    #field_into_bytes_calls
                )*
                result
            }
        }

        impl #impl_gen ::light_hasher::DataHasher for #struct_name #type_gen #where_clause {
            fn hash<H: light_hasher::Hasher>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::errors::HasherError> {
                use ::light_hasher::bytes::AsByteVec;
                use ::light_hasher::DataHasher;

                let bytes = self.as_byte_vec();
                let nested_bytes: Vec<_> = bytes.iter().map(|v| v.as_slice()).collect();
                H::hashv(&nested_bytes)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;
    use prettyplease::unparse;

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

        let formatted_output = unparse(&syn::parse2(output).unwrap());

        assert!(formatted_output.contains("impl ::light_hasher::bytes::AsByteVec for MyAccount"));
    }
    #[test]
    fn test_nested_struct() {
        let input: ItemStruct = parse_quote! {
            struct OuterStruct {
                a: u32,
                #[nested]
                b: InnerStruct,
            }
        };

        let output = hasher(input).unwrap();
  
        
        let syntax_tree: syn::File = syn::parse2(output).unwrap();
        let formatted_output = unparse(&syntax_tree);
        println!("{}", formatted_output);
        assert!(formatted_output.contains("impl ::light_hasher::bytes::AsByteVec for OuterStruct"));
        assert!(formatted_output.contains("impl ::light_hasher::DataHasher for OuterStruct"));
        assert!(formatted_output.contains("result.extend(self.b.as_byte_vec());"));
    }

    #[test]
    fn test_nested_struct_with_attributes() {
        let input: ItemStruct = parse_quote! {
            struct OuterStruct {
                a: u32,
                #[truncate]
                b: InnerStruct,
                #[skip]
                c: SkippedStruct,
                #[nested]
                d: NestedStruct,
            }
        };

        let output = hasher(input).unwrap();
        let syntax_tree: syn::File = syn::parse2(output).unwrap();
        let formatted_output = unparse(&syntax_tree);

        assert!(formatted_output.contains("impl ::light_hasher::bytes::AsByteVec for OuterStruct"));
        assert!(formatted_output.contains("impl ::light_hasher::DataHasher for OuterStruct"));
        assert!(formatted_output.contains("truncate"));
        assert!(formatted_output.contains("self.d.as_byte_vec()"));
        assert!(!formatted_output.contains("SkippedStruct"));
    }
}
