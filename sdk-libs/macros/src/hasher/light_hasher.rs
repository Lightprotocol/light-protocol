use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, ItemStruct, Result};

use crate::hasher::{
    data_hasher::generate_data_hasher_impl,
    field_processor::{process_field, FieldProcessingContext},
    input_validator::{get_field_attribute, validate_input, FieldAttribute},
    to_byte_array::generate_to_byte_array_impl,
};

/// - ToByteArray:
///     1. ToByteArray -> [u8;32]
///     2. ToByteArrays -> [[u8;32]; NUM_FIELDS]
///     3. const NumFields -> usize (can be used to get ToByteArrays)
/// - DataHasher Hash -> [u8;32]
///
/// - Attribute Macros:
///     1. hash
///        hash to bn254 field size (less than 254 bit), hash with keccak256 and truncate to 31 bytes
///     2. skip
///        ignore field
///     3. flatten
///        flatten nested struct or vector
///
/// Derive macro for ToByteArray
/// - Struct:
///   - every field must implement ToByteArray
///   - impl ToByteArray for Struct -> returns hash of all fields
///   - impl DataHasher for Struct -> returns hash of all fields
/// - Options (primitive types PT):
///     - Option<PT> -> [u8;32] -> Some: [32 - type_bytes_len..] 32 - index type_bytes_len -1 = [1] (BE prefix) , None: [0;32]
/// - Option (General):
///     - Option<T> T must implement Hash -> Some: Hash(T::hash), None: [0u8;32]
/// - Nested Structs:
///     - to_byte_array -> hash of all fields
/// - Arrays (u8):
///     1. LEN < 32 implementation of ToByteArray is provided
///     2. LEN >= 32  needs to be handled (can be truncated or implement custom ToByteArray)
/// - Arrays:
///     1. if elements implement ToByteArray and are less than 13, hash of all elements
///     2. More elements than 13 -> manual implementation or hash to field size
/// - Vec<T>:
///     - we do not provide a blanket implementation since it could fail in production once a vector fills up
///     - users need to hash to field size or do a manual implementation
/// - Strings:
///     - we do not provide a blanket implementation since it could fail in production once a string fills up
///     - users need to hash to field size or do a manual implementation
/// - Enums, References, SmartPointers:
///     - Not supported
pub(crate) fn derive_light_hasher(input: ItemStruct) -> Result<TokenStream> {
    // Validate the input structure
    validate_input(&input)?;

    let generics = input.generics.clone();

    // After validation, we know this is a named field struct
    let fields = match &input.fields {
        Fields::Named(fields) => fields.clone(),
        _ => unreachable!("Validation should have caught this"),
    };

    let field_count = fields.named.len();
    let flatten_field_exists = fields
        .named
        .iter()
        .any(|field| get_field_attribute(field) == FieldAttribute::Flatten);

    // Create processing context
    let mut context = FieldProcessingContext::new(flatten_field_exists);

    // Process each field
    fields.named.iter().enumerate().for_each(|(i, field)| {
        process_field(field, i, &mut context);
    });

    let to_byte_array_impl =
        generate_to_byte_array_impl(&input.ident, &generics, field_count, &context)?;

    let data_hasher_impl = generate_data_hasher_impl(&input.ident, &generics, &context)?;

    // Combine implementations
    Ok(quote! {
        #to_byte_array_impl

        #data_hasher_impl
    })
}

#[cfg(test)]
mod tests {
    use prettyplease::unparse;
    use syn::{parse_quote, ItemStruct};

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

        let output = derive_light_hasher(input).unwrap();
        let formatted_output = unparse(&syn::parse2(output).unwrap());

        const EXPECTED_OUTPUT: &str = r#"impl ::light_hasher::to_byte_array::ToByteArray for MyAccount {
    const NUM_FIELDS: usize = 4usize;
    fn to_byte_array(
        &self,
    ) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError> {
        ::light_hasher::DataHasher::hash::<::light_hasher::Poseidon>(self)
    }
}
impl ::light_hasher::DataHasher for MyAccount {
    fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
    where
        H: ::light_hasher::Hasher,
    {
        use ::light_hasher::DataHasher;
        use ::light_hasher::Hasher;
        use ::light_hasher::to_byte_array::ToByteArray;
        #[cfg(debug_assertions)]
       {
            if std::env::var("RUST_BACKTRACE").is_ok() {
                let debug_prints: Vec<[u8;32]> = vec![
                    self.a.to_byte_array()?,
                    self.b.to_byte_array()?,
                    self.c.to_byte_array()?,
                    self.d.to_byte_array()?,
                ];
            }
           println!("DataHasher::hash inputs {:?}", debug_prints);
       }
        H::hashv(
            &[
                self.a.to_byte_array()?.as_slice(),
                self.b.to_byte_array()?.as_slice(),
                self.c.to_byte_array()?.as_slice(),
                self.d.to_byte_array()?.as_slice(),
            ],
        )
    }
}"#;

        let expected_syntax = syn::parse_str::<syn::File>(EXPECTED_OUTPUT).unwrap();
        let expected_formatted = unparse(&expected_syntax);
        assert_eq!(formatted_output, expected_formatted);
    }

    #[test]
    fn test_option_handling() {
        let input: ItemStruct = parse_quote! {
            struct OptionStruct {
                a: Option<u32>,
                b: Option<String>,
            }
        };

        let output = derive_light_hasher(input).unwrap();
        let formatted_output = unparse(&syn::parse2(output).unwrap());

        const EXPECTED_OUTPUT: &str = r#"impl ::light_hasher::to_byte_array::ToByteArray for OptionStruct {
    const NUM_FIELDS: usize = 2usize;
    fn to_byte_array(
        &self,
    ) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError> {
        ::light_hasher::DataHasher::hash::<::light_hasher::Poseidon>(self)
    }
}
impl ::light_hasher::DataHasher for OptionStruct {
    fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
    where
        H: ::light_hasher::Hasher,
    {
        use ::light_hasher::DataHasher;
        use ::light_hasher::Hasher;
        use ::light_hasher::to_byte_array::ToByteArray;
        #[cfg(debug_assertions)]
       {
            if std::env::var("RUST_BACKTRACE").is_ok() {
                let debug_prints: Vec<[u8;32]> = vec![
                    self.a.to_byte_array()?,
                    self.b.to_byte_array()?,
                ];
                println!("DataHasher::hash inputs {:?}", debug_prints);
            }
       }
        H::hashv(
            &[self.a.to_byte_array()?.as_slice(), self.b.to_byte_array()?.as_slice()],
        )
    }
}"#;

        let expected_syntax = syn::parse_str::<syn::File>(EXPECTED_OUTPUT).unwrap();
        let expected_formatted = unparse(&expected_syntax);
        assert_eq!(formatted_output, expected_formatted);
    }

    #[test]
    fn test_truncate_option() {
        let input: ItemStruct = parse_quote! {
            struct TruncateOptionStruct {
                #[hash]
                a: Option<String>,
            }
        };

        let output = derive_light_hasher(input).unwrap();
        let formatted_output = unparse(&syn::parse2(output).unwrap());

        const EXPECTED_OUTPUT: &str = r#"impl ::light_hasher::to_byte_array::ToByteArray for TruncateOptionStruct {
    const NUM_FIELDS: usize = 1usize;
    fn to_byte_array(
        &self,
    ) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError> {
        use ::light_hasher::hash_to_field_size::HashToFieldSize;
        Ok(
            if let Some(a) =  &self.a {
                let result = a.hash_to_field_size()?;
                if result == [0u8; 32] {
                    return Err(
                        ::light_hasher::errors::HasherError::OptionHashToFieldSizeZero,
                    );
                }
                result
            } else {
                [0u8; 32]
            }
        )
    }
}
impl ::light_hasher::DataHasher for TruncateOptionStruct {
    fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
    where
        H: ::light_hasher::Hasher,
    {
        use ::light_hasher::DataHasher;
        use ::light_hasher::Hasher;
        use ::light_hasher::to_byte_array::ToByteArray;
        use ::light_hasher::hash_to_field_size::HashToFieldSize;
        #[cfg(debug_assertions)]
       {
            if std::env::var("RUST_BACKTRACE").is_ok() {
                let debug_prints: Vec<[u8; 32]> = vec![
                    if let Some(a) = & self.a { let result = a.hash_to_field_size() ?; if
                    result == [0u8; 32] { return
                    Err(::light_hasher::errors::HasherError::OptionHashToFieldSizeZero); }
                    result } else { [0u8; 32] },
                ];
                println!("DataHasher::hash inputs {:?}", debug_prints);
            }
       }
        H::hashv(
            &[
                if let Some(a) = &self.a {
                    let result = a.hash_to_field_size()?;
                    if result == [0u8; 32] {
                        return Err(
                            ::light_hasher::errors::HasherError::OptionHashToFieldSizeZero,
                        );
                    }
                    result
                } else {
                    [0u8; 32]
                }

                .as_slice(),
            ],
        )
    }
}"#;

        let expected_syntax = syn::parse_str::<syn::File>(EXPECTED_OUTPUT).unwrap();
        let expected_formatted = unparse(&expected_syntax);
        assert_eq!(formatted_output, expected_formatted);
    }

    #[test]
    fn test_mixed_attributes() {
        let input: ItemStruct = parse_quote! {
            struct MixedStruct {
                a: u32,
                #[hash]
                b: String,
                c: Option<u64>,
            }
        };

        let output = derive_light_hasher(input).unwrap();
        let formatted_output = unparse(&syn::parse2(output).unwrap());

        const EXPECTED_OUTPUT: &str = r#"impl ::light_hasher::to_byte_array::ToByteArray for MixedStruct {
    const NUM_FIELDS: usize = 3usize;
    fn to_byte_array(
        &self,
    ) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError> {
        ::light_hasher::DataHasher::hash::<::light_hasher::Poseidon>(self)
    }
}
impl ::light_hasher::DataHasher for MixedStruct {
    fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
    where
        H: ::light_hasher::Hasher,
    {
        use ::light_hasher::DataHasher;
        use ::light_hasher::Hasher;
        use ::light_hasher::to_byte_array::ToByteArray;
        use ::light_hasher::hash_to_field_size::HashToFieldSize;
        #[cfg(debug_assertions)]
       {
            if std::env::var("RUST_BACKTRACE").is_ok() {
                let debug_prints: Vec<[u8;32]> = vec![
                    self.a.to_byte_array()?,
                    self.b.hash_to_field_size()?,
                    self.c.to_byte_array()?,
                ];
                println!("DataHasher::hash inputs {:?}", debug_prints);
            }
       }
        H::hashv(
            &[
                self.a.to_byte_array()?.as_slice(),
                self.b.hash_to_field_size()?.as_slice(),
                self.c.to_byte_array()?.as_slice(),
            ],
        )
    }
}"#;

        let expected_syntax = syn::parse_str::<syn::File>(EXPECTED_OUTPUT).unwrap();
        let expected_formatted = unparse(&expected_syntax);
        assert_eq!(formatted_output, expected_formatted);
    }

    #[test]
    fn test_nested_struct() {
        let input: ItemStruct = parse_quote! {
            struct OuterStruct {
                a: u32,
                b: InnerStruct,
            }
        };

        let output = derive_light_hasher(input).unwrap();
        let formatted_output = unparse(&syn::parse2(output).unwrap());

        const EXPECTED_OUTPUT: &str = r#"impl ::light_hasher::to_byte_array::ToByteArray for OuterStruct {
    const NUM_FIELDS: usize = 2usize;
    fn to_byte_array(
        &self,
    ) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError> {
        ::light_hasher::DataHasher::hash::<::light_hasher::Poseidon>(self)
    }
}
impl ::light_hasher::DataHasher for OuterStruct {
    fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
    where
        H: ::light_hasher::Hasher,
    {
        use ::light_hasher::DataHasher;
        use ::light_hasher::Hasher;
        use ::light_hasher::to_byte_array::ToByteArray;
        #[cfg(debug_assertions)]
       {
            if std::env::var("RUST_BACKTRACE").is_ok() {
                let debug_prints: Vec<[u8;32]> = vec![
                    self.a.to_byte_array()?,
                    self.b.to_byte_array()?,
                ];
                println!("DataHasher::hash inputs {:?}", debug_prints);
            }
       }
        H::hashv(
            &[self.a.to_byte_array()?.as_slice(), self.b.to_byte_array()?.as_slice()],
        )
    }
}"#;
        // Format both the expected and actual output using prettyplease
        let expected_syntax = syn::parse_str::<syn::File>(EXPECTED_OUTPUT).unwrap();
        let expected_formatted = unparse(&expected_syntax);
        assert_eq!(formatted_output, expected_formatted);
    }

    #[test]
    fn test_option_validation() {
        let input: ItemStruct = parse_quote! {
            struct OptionStruct {
                a: Option<u32>,
                #[hash]
                b: Option<String>,
            }
        };
        assert!(derive_light_hasher(input).is_ok());

        // In the new implementation, we don't have the nested attribute anymore
        // and there shouldn't be an error when using truncate with Option<u32>
        let input: ItemStruct = parse_quote! {
            struct ValidStruct {
                #[hash]
                a: Option<u32>,
            }
        };
        assert!(derive_light_hasher(input).is_ok());
    }
}
