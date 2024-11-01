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

    fn is_option_type(ty: &syn::Type) -> bool {
        if let syn::Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                return segment.ident == "Option";
            }
        }
        false
    }

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
            let is_option = is_option_type(&field.ty);


            Ok(if nested {
                if is_option {
                    quote! {
                        match &self.#field_name {
                            Some(value) => {
                                let mut nested_hash = ::light_hasher::DataHasher::hash::<::light_hasher::Poseidon>(value)
                                    .expect("Failed to hash nested field");
                                nested_hash[0] = 1; // Mark as Some
                                result.push(nested_hash.to_vec());
                            }
                            None => {
                                result.push([0u8; 32].to_vec());
                            }
                        }
                    }
                } else {
                    quote! {
                        let nested_hash = ::light_hasher::DataHasher::hash::<::light_hasher::Poseidon>(&self.#field_name)
                            .expect("Failed to hash nested field");
                        result.push(nested_hash.to_vec());
                    }
                }
            } else if is_option && truncate {
                quote! {
                    match &self.#field_name {
                        Some(value) => {
                            let (mut bytes, _) = ::light_utils::hash_to_bn254_field_size_be(&value.as_bytes())
                                .expect("Could not truncate to BN254 field size");
                            bytes[0] = 1; // Mark as Some
                            result.push(bytes.to_vec());
                        }
                        None => {
                            result.push([0u8; 32].to_vec());
                        }
                    }
                }
            } else if is_option {
                quote! {
                    match &self.#field_name {
                        Some(value) => {
                            let bytes = value.to_le_bytes();
                            // if bytes.len() == 32 {
                            //     if !::light_utils::is_smaller_than_bn254_field_size_be(&bytes) {
                            //         panic!("Option value too large - use #[truncate] to handle large values");
                            //     }
                            // }
                            let mut result_bytes = vec![1u8]; // Mark as Some
                            result_bytes.extend_from_slice(&bytes);
                            result.push(result_bytes);
                        }
                        None => {
                            result.push(vec![0]); // Just a single byte for None
                        }
                    }
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
            })
        })
        .collect::<Result<Vec<_>>>()?;

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
    #[test]
    fn test_option_handling() {
        // Test normal Option handling
        let input: ItemStruct = parse_quote! {
            struct OptionStruct {
                a: Option<u32>,
                b: Option<String>,
            }
        };

        let output = hasher(input).unwrap();
        let syntax_tree = syn::parse2(output).unwrap();
        let formatted_output = unparse(&syntax_tree);
        println!("{}", formatted_output);

        assert!(formatted_output.contains("match &self"));
        assert!(formatted_output.contains("let mut result_bytes = vec![1u8]"));
        assert!(formatted_output.contains("result.push(vec![0])"));
        assert!(formatted_output.contains("result_bytes.extend_from_slice(&bytes)"));

        // Test Option with truncate for large values
        let input: ItemStruct = parse_quote! {
            struct LargeOptionStruct {
                #[truncate]
                a: Option<[u8; 64]>, // Deliberately larger than 32 bytes
            }
        };
        assert!(hasher(input).is_ok());
    }
    #[test]
    fn test_nested_struct_with_option() {
        let input: ItemStruct = parse_quote! {
            struct OuterStruct {
                a: u32,
                #[nested]
                b: InnerStruct,
                #[truncate]
                c: Option<u64>,
            }
        };

        let output = hasher(input).unwrap();
        let syntax_tree = syn::parse2(output).unwrap();
        let formatted_output = unparse(&syntax_tree);

        assert!(formatted_output.contains("impl ::light_hasher::bytes::AsByteVec for OuterStruct"));
        assert!(formatted_output.contains("match &self.c"));
        assert!(formatted_output.contains("result.push([0u8; 32].to_vec())"));
    }

    #[test]
    fn test_option_validation() {
        // Test nested Option<Struct>
        let input: ItemStruct = parse_quote! {
            struct NestedOptionStruct {
                #[nested]
                opt: Option<InnerStruct>,
            }
        };
        assert!(hasher(input).is_ok());

        let input: ItemStruct = parse_quote! {
            struct MixedOptionsStruct {
                #[nested]
                a: Option<InnerStruct>,
                #[truncate]
                b: Option<String>,
                c: Option<u32>,
            }
        };
        assert!(hasher(input).is_ok());
    }
}
