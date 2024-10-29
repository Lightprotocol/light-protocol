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
                    // For nested fields, hash them first
                    let nested_hash = ::light_hasher::DataHasher::hash::<::light_hasher::Poseidon>(&self.#field_name)
                        .expect("Failed to hash nested field");
                    result.push(nested_hash.to_vec());
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
            fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
            where
                H: ::light_hasher::Hasher
            {
                use ::light_hasher::bytes::AsByteVec;
                use ::light_hasher::DataHasher;
                use ::light_hasher::Hasher;

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
    use prettyplease::unparse;
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

        assert!(formatted_output.contains("impl ::light_hasher::bytes::AsByteVec for OuterStruct"));
        assert!(formatted_output.contains("impl ::light_hasher::DataHasher for OuterStruct"));
        assert!(formatted_output.contains("let nested_hash = ::light_hasher::DataHasher::hash::<"));
        assert!(formatted_output.contains("::light_hasher::Poseidon,"));
        assert!(formatted_output.contains(">(&self.b)"));
        assert!(formatted_output.contains("result.push(nested_hash.to_vec());"));
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
        let syntax_tree = syn::parse2(output).unwrap();
        let formatted_output = unparse(&syntax_tree);
        assert!(formatted_output.contains("impl ::light_hasher::bytes::AsByteVec for OuterStruct"));
        assert!(formatted_output.contains("impl ::light_hasher::DataHasher for OuterStruct"));
        assert!(formatted_output.contains("result.extend(self.a.as_byte_vec())"));
        assert!(formatted_output
            .contains("let (bytes, _) = ::light_utils::hash_to_bn254_field_size_be(bytes)"));
        assert!(formatted_output.contains("let nested_hash = ::light_hasher::DataHasher::hash::<"));
        assert!(formatted_output.contains("::light_hasher::Poseidon,"));
        assert!(formatted_output.contains(">(&self.d)"));
        assert!(formatted_output.contains(".expect(\"Failed to hash nested field\")"));
        assert!(!formatted_output.contains("c: SkippedStruct"));
    }
}
