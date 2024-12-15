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
        .map(|field| {
            let field_name = &field.ident;
            let truncate = field.attrs.iter().any(|attr| attr.path().is_ident("truncate"));
            let nested = field.attrs.iter().any(|attr| attr.path().is_ident("nested")); 
            if truncate && nested {
                return Err(Error::new_spanned(
                    field,
                    "Field cannot have both #[nested] and #[truncate] attributes",
                ));
            }

            let is_option = is_option_type(&field.ty);

            Ok(if nested {
                if is_option {
                    quote! {
                        match &self.#field_name {
                            Some(value) => {
                                let nested_hash = ::light_hasher::DataHasher::hash::<::light_hasher::Poseidon>(value)
                                    .expect("Failed to hash nested field");
                                result.push(nested_hash.to_vec());
                            }
                            None => {
                                result.push(vec![0]);
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
                            let bytes = value.as_byte_vec().into_iter().flatten().collect::<Vec<_>>();
                            let (bytes, _) = ::light_utils::hash_to_bn254_field_size_be(&bytes)
                                .expect("Could not truncate to BN254 field size");
                            result.push(bytes.to_vec());
                        }
                        None => {
                            result.push(vec![0]);
                        }
                    }
                }
            } else if is_option {
                quote! {
                    match &self.#field_name {
                        Some(value) => {
                            let mut bytes = vec![1u8];
                            bytes.extend(value.as_byte_vec().into_iter().flatten());
                            result.push(bytes);
                        }
                        None => {
                            result.push(vec![0]);
                        }
                    }
                }
            } else if truncate {
                quote! {
                    let bytes = {
                        let value = &self.#field_name;
                        let bytes = value.as_byte_vec();
                        let (hash, _) = ::light_utils::hash_to_bn254_field_size_be(&bytes[0])
                            .expect("Could not truncate to BN254 field size");
                        result.push(hash.to_vec());
                    };
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
    use prettyplease::unparse;
    use syn::parse_quote;

    use super::*;

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
        let expected_output = r#"impl ::light_hasher::bytes::AsByteVec for MyAccount {
    fn as_byte_vec(&self) -> Vec<Vec<u8>> {
        use ::light_hasher::bytes::AsByteVec;
        let mut result: Vec<Vec<u8>> = Vec::new();
        result.extend(self.a.as_byte_vec());
        result.extend(self.b.as_byte_vec());
        result.extend(self.c.as_byte_vec());
        result.extend(self.d.as_byte_vec());
        result
    }
}
impl ::light_hasher::DataHasher for MyAccount {
    fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
    where
        H: ::light_hasher::Hasher,
    {
        use ::light_hasher::bytes::AsByteVec;
        use ::light_hasher::DataHasher;
        use ::light_hasher::Hasher;
        let bytes = self.as_byte_vec();
        let nested_bytes: Vec<_> = bytes.iter().map(|v| v.as_slice()).collect();
        H::hashv(&nested_bytes)
    }
}"#;
        assert_eq!(formatted_output.trim(), expected_output.trim());
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

        let expected_output = r#"impl ::light_hasher::bytes::AsByteVec for OuterStruct {
    fn as_byte_vec(&self) -> Vec<Vec<u8>> {
        use ::light_hasher::bytes::AsByteVec;
        let mut result: Vec<Vec<u8>> = Vec::new();
        result.extend(self.a.as_byte_vec());
        let nested_hash = ::light_hasher::DataHasher::hash::<
            ::light_hasher::Poseidon,
        >(&self.b)
            .expect("Failed to hash nested field");
        result.push(nested_hash.to_vec());
        result
    }
}
impl ::light_hasher::DataHasher for OuterStruct {
    fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
    where
        H: ::light_hasher::Hasher,
    {
        use ::light_hasher::bytes::AsByteVec;
        use ::light_hasher::DataHasher;
        use ::light_hasher::Hasher;
        let bytes = self.as_byte_vec();
        let nested_bytes: Vec<_> = bytes.iter().map(|v| v.as_slice()).collect();
        H::hashv(&nested_bytes)
    }
}"#;
        assert_eq!(formatted_output.trim(), expected_output.trim());
    }
    #[test]
    fn test_nested_struct_with_attributes() {
        let input: ItemStruct = parse_quote! {
            struct OuterStruct {
                a: u32,
                #[truncate]
                b: InnerStruct,
                #[nested]
                d: NestedStruct,
            }
        };

        let output = hasher(input).unwrap();
        let syntax_tree = syn::parse2(output).unwrap();
        let formatted_output = unparse(&syntax_tree);

        let expected_output = r#"impl ::light_hasher::bytes::AsByteVec for OuterStruct {
    fn as_byte_vec(&self) -> Vec<Vec<u8>> {
        use ::light_hasher::bytes::AsByteVec;
        let mut result: Vec<Vec<u8>> = Vec::new();
        result.extend(self.a.as_byte_vec());
        let bytes = {
            let value = &self.b;
            let bytes = value.as_byte_vec();
            let (hash, _) = ::light_utils::hash_to_bn254_field_size_be(&bytes[0])
                .expect("Could not truncate to BN254 field size");
            result.push(hash.to_vec());
        };
        let nested_hash = ::light_hasher::DataHasher::hash::<
            ::light_hasher::Poseidon,
        >(&self.d)
            .expect("Failed to hash nested field");
        result.push(nested_hash.to_vec());
        result
    }
}"#;

        assert_eq!(formatted_output.contains(expected_output), true);
    }
    #[test]
    fn test_option_handling() {
        let input: ItemStruct = parse_quote! {
            struct OptionStruct {
                a: Option<u32>,
                b: Option<String>,
            }
        };

        let output = hasher(input).unwrap();
        let syntax_tree = syn::parse2(output).unwrap();
        let formatted_output = unparse(&syntax_tree);

        let expected_output = r#"impl ::light_hasher::bytes::AsByteVec for OptionStruct {
    fn as_byte_vec(&self) -> Vec<Vec<u8>> {
        use ::light_hasher::bytes::AsByteVec;
        let mut result: Vec<Vec<u8>> = Vec::new();
        match &self.a {
            Some(value) => {
                let mut bytes = vec![1u8];
                bytes.extend(value.as_byte_vec().into_iter().flatten());
                result.push(bytes);
            }
            None => {
                result.push(vec![0]);
            }
        }
        match &self.b {
            Some(value) => {
                let mut bytes = vec![1u8];
                bytes.extend(value.as_byte_vec().into_iter().flatten());
                result.push(bytes);
            }
            None => {
                result.push(vec![0]);
            }
        }
        result
    }
}
impl ::light_hasher::DataHasher for OptionStruct {
    fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
    where
        H: ::light_hasher::Hasher,
    {
        use ::light_hasher::bytes::AsByteVec;
        use ::light_hasher::DataHasher;
        use ::light_hasher::Hasher;
        let bytes = self.as_byte_vec();
        let nested_bytes: Vec<_> = bytes.iter().map(|v| v.as_slice()).collect();
        H::hashv(&nested_bytes)
    }
}"#;
        assert_eq!(formatted_output.trim(), expected_output.trim());
    }
    #[test]
    fn test_option_with_attributes() {
        // Test truncate option
        let input: ItemStruct = parse_quote! {
            struct TruncateOptionStruct {
                #[truncate]
                a: Option<String>,
            }
        };

        let output = hasher(input).unwrap();
        let formatted_output = unparse(&syn::parse2(output).unwrap());

        let expected_truncate = r#"impl ::light_hasher::bytes::AsByteVec for TruncateOptionStruct {
    fn as_byte_vec(&self) -> Vec<Vec<u8>> {
        use ::light_hasher::bytes::AsByteVec;
        let mut result: Vec<Vec<u8>> = Vec::new();
        match &self.a {
            Some(value) => {
                let bytes = value
                    .as_byte_vec()
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();
                let (bytes, _) = ::light_utils::hash_to_bn254_field_size_be(&bytes)
                    .expect("Could not truncate to BN254 field size");
                result.push(bytes.to_vec());
            }
            None => {
                result.push(vec![0]);
            }
        }
        result
    }
}
impl ::light_hasher::DataHasher for TruncateOptionStruct {
    fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
    where
        H: ::light_hasher::Hasher,
    {
        use ::light_hasher::bytes::AsByteVec;
        use ::light_hasher::DataHasher;
        use ::light_hasher::Hasher;
        let bytes = self.as_byte_vec();
        let nested_bytes: Vec<_> = bytes.iter().map(|v| v.as_slice()).collect();
        H::hashv(&nested_bytes)
    }
}"#;
        assert_eq!(formatted_output.trim(), expected_truncate.trim());

        // Test nested option
        let input: ItemStruct = parse_quote! {
            struct NestedOptionStruct {
                #[nested]
                a: Option<InnerStruct>,
            }
        };

        let output = hasher(input).unwrap();
        let formatted_output = unparse(&syn::parse2(output).unwrap());

        let expected_nested = r#"impl ::light_hasher::bytes::AsByteVec for NestedOptionStruct {
    fn as_byte_vec(&self) -> Vec<Vec<u8>> {
        use ::light_hasher::bytes::AsByteVec;
        let mut result: Vec<Vec<u8>> = Vec::new();
        match &self.a {
            Some(value) => {
                let nested_hash = ::light_hasher::DataHasher::hash::<
                    ::light_hasher::Poseidon,
                >(value)
                    .expect("Failed to hash nested field");
                result.push(nested_hash.to_vec());
            }
            None => {
                result.push(vec![0]);
            }
        }
        result
    }
}
impl ::light_hasher::DataHasher for NestedOptionStruct {
    fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
    where
        H: ::light_hasher::Hasher,
    {
        use ::light_hasher::bytes::AsByteVec;
        use ::light_hasher::DataHasher;
        use ::light_hasher::Hasher;
        let bytes = self.as_byte_vec();
        let nested_bytes: Vec<_> = bytes.iter().map(|v| v.as_slice()).collect();
        H::hashv(&nested_bytes)
    }
}"#;
        assert_eq!(formatted_output.trim(), expected_nested.trim());
    }

    #[test]
    fn test_mixed_options() {
        let input: ItemStruct = parse_quote! {
            struct MixedOptionsStruct {
                #[nested]
                a: Option<InnerStruct>,
                #[truncate]
                b: Option<String>,
                c: Option<u32>,
            }
        };

        let output = hasher(input).unwrap();
        let formatted_output = unparse(&syn::parse2(output).unwrap());

        let expected_output = r#"impl ::light_hasher::bytes::AsByteVec for MixedOptionsStruct {
    fn as_byte_vec(&self) -> Vec<Vec<u8>> {
        use ::light_hasher::bytes::AsByteVec;
        let mut result: Vec<Vec<u8>> = Vec::new();
        match &self.a {
            Some(value) => {
                let nested_hash = ::light_hasher::DataHasher::hash::<
                    ::light_hasher::Poseidon,
                >(value)
                    .expect("Failed to hash nested field");
                result.push(nested_hash.to_vec());
            }
            None => {
                result.push(vec![0]);
            }
        }
        match &self.b {
            Some(value) => {
                let bytes = value
                    .as_byte_vec()
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();
                let (bytes, _) = ::light_utils::hash_to_bn254_field_size_be(&bytes)
                    .expect("Could not truncate to BN254 field size");
                result.push(bytes.to_vec());
            }
            None => {
                result.push(vec![0]);
            }
        }
        match &self.c {
            Some(value) => {
                let mut bytes = vec![1u8];
                bytes.extend(value.as_byte_vec().into_iter().flatten());
                result.push(bytes);
            }
            None => {
                result.push(vec![0]);
            }
        }
        result
    }
}
impl ::light_hasher::DataHasher for MixedOptionsStruct {
    fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
    where
        H: ::light_hasher::Hasher,
    {
        use ::light_hasher::bytes::AsByteVec;
        use ::light_hasher::DataHasher;
        use ::light_hasher::Hasher;
        let bytes = self.as_byte_vec();
        let nested_bytes: Vec<_> = bytes.iter().map(|v| v.as_slice()).collect();
        H::hashv(&nested_bytes)
    }
}"#;

        assert_eq!(formatted_output.trim(), expected_output.trim());
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

        let expected_output = r#"impl ::light_hasher::bytes::AsByteVec for OuterStruct {
    fn as_byte_vec(&self) -> Vec<Vec<u8>> {
        use ::light_hasher::bytes::AsByteVec;
        let mut result: Vec<Vec<u8>> = Vec::new();
        result.extend(self.a.as_byte_vec());
        let nested_hash = ::light_hasher::DataHasher::hash::<
            ::light_hasher::Poseidon,
        >(&self.b)
            .expect("Failed to hash nested field");
        result.push(nested_hash.to_vec());
        match &self.c {
            Some(value) => {
                let bytes = value
                    .as_byte_vec()
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();
                let (bytes, _) = ::light_utils::hash_to_bn254_field_size_be(&bytes)
                    .expect("Could not truncate to BN254 field size");
                result.push(bytes.to_vec());
            }
            None => {
                result.push(vec![0]);
            }
        }
        result
    }
}
impl ::light_hasher::DataHasher for OuterStruct {
    fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
    where
        H: ::light_hasher::Hasher,
    {
        use ::light_hasher::bytes::AsByteVec;
        use ::light_hasher::DataHasher;
        use ::light_hasher::Hasher;
        let bytes = self.as_byte_vec();
        let nested_bytes: Vec<_> = bytes.iter().map(|v| v.as_slice()).collect();
        H::hashv(&nested_bytes)
    }
}"#;

        assert_eq!(formatted_output.trim(), expected_output.trim());
    }

    #[test]
    fn test_option_validation() {
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
