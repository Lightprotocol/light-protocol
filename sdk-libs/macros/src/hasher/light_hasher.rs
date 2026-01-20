use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, ItemStruct, Result};

use crate::hasher::{
    data_hasher::{generate_data_hasher_impl, generate_data_hasher_impl_sha},
    field_processor::{process_field, FieldProcessingContext},
    input_validator::{get_field_attribute, validate_input, validate_input_sha, FieldAttribute},
    to_byte_array::{generate_to_byte_array_impl_sha, generate_to_byte_array_impl_with_hasher},
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
    derive_light_hasher_with_hasher(input, &quote!(::light_hasher::Poseidon))
}

pub(crate) fn derive_light_hasher_sha(input: ItemStruct) -> Result<TokenStream> {
    // Use SHA256-specific validation (no field count limits)
    validate_input_sha(&input)?;

    let generics = input.generics.clone();

    let fields = match &input.fields {
        Fields::Named(fields) => fields.clone(),
        _ => unreachable!("Validation should have caught this"),
    };

    let field_count = fields.named.len();

    let to_byte_array_impl = generate_to_byte_array_impl_sha(&input.ident, &generics, field_count)?;
    let data_hasher_impl = generate_data_hasher_impl_sha(&input.ident, &generics)?;

    Ok(quote! {
        #to_byte_array_impl

        #data_hasher_impl
    })
}

fn derive_light_hasher_with_hasher(input: ItemStruct, hasher: &TokenStream) -> Result<TokenStream> {
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

    let to_byte_array_impl = generate_to_byte_array_impl_with_hasher(
        &input.ident,
        &generics,
        field_count,
        &context,
        hasher,
    )?;

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
        use ::light_hasher::to_byte_array::ToByteArray;
        use ::light_hasher::hash_to_field_size::HashToFieldSize;
        use ::light_hasher::Hasher;
        let mut result = ::light_hasher::Poseidon::hashv(
            &[
                self.a.to_byte_array()?.as_slice(),
                self.b.to_byte_array()?.as_slice(),
                self.c.to_byte_array()?.as_slice(),
                self.d.to_byte_array()?.as_slice(),
            ],
        )?;
        if ::light_hasher::Poseidon::ID != ::light_hasher::Poseidon::ID {
            result[0] = 0;
        }
        Ok(result)
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
                let debug_prints: Vec<[u8; 32]> = vec![
                    self.a.to_byte_array() ?, self.b.to_byte_array() ?, self.c
                    .to_byte_array() ?, self.d.to_byte_array() ?,
                ];
                println!("DataHasher::hash inputs {:?}", debug_prints);
            }
        }
        let mut result = H::hashv(
            &[
                self.a.to_byte_array()?.as_slice(),
                self.b.to_byte_array()?.as_slice(),
                self.c.to_byte_array()?.as_slice(),
                self.d.to_byte_array()?.as_slice(),
            ],
        )?;
        if H::ID != ::light_hasher::Poseidon::ID {
            result[0] = 0;
        }
        Ok(result)
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
        use ::light_hasher::to_byte_array::ToByteArray;
        use ::light_hasher::hash_to_field_size::HashToFieldSize;
        use ::light_hasher::Hasher;
        let mut result = ::light_hasher::Poseidon::hashv(
            &[self.a.to_byte_array()?.as_slice(), self.b.to_byte_array()?.as_slice()],
        )?;
        if ::light_hasher::Poseidon::ID != ::light_hasher::Poseidon::ID {
            result[0] = 0;
        }
        Ok(result)
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
        let mut result = H::hashv(
            &[self.a.to_byte_array()?.as_slice(), self.b.to_byte_array()?.as_slice()],
        )?;
        if H::ID != ::light_hasher::Poseidon::ID {
            result[0] = 0;
        }
        Ok(result)
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
    const NUM_FIELDS: usize = 1;
    fn to_byte_array(
        &self,
    ) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError> {
        use ::light_hasher::to_byte_array::ToByteArray;
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
                let debug_prints: Vec<[u8;32]> = vec![
                    if let Some(a) = & self.a { let result = a.hash_to_field_size() ?; if
                    result == [0u8; 32] { return
                    Err(::light_hasher::errors::HasherError::OptionHashToFieldSizeZero); }
                    result } else { [0u8; 32] },
                ];
                println!("DataHasher::hash inputs {:?}", debug_prints);
            }
       }
        let mut result = H::hashv(
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
        )?;
        if H::ID != ::light_hasher::Poseidon::ID {
            result[0] = 0;
        }
        Ok(result)
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
        use ::light_hasher::to_byte_array::ToByteArray;
        use ::light_hasher::hash_to_field_size::HashToFieldSize;
        use ::light_hasher::Hasher;
        let mut result = ::light_hasher::Poseidon::hashv(
            &[
                self.a.to_byte_array()?.as_slice(),
                self.b.hash_to_field_size()?.as_slice(),
                self.c.to_byte_array()?.as_slice(),
            ],
        )?;
        if ::light_hasher::Poseidon::ID != ::light_hasher::Poseidon::ID {
            result[0] = 0;
        }
        Ok(result)
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
        let mut result = H::hashv(
            &[
                self.a.to_byte_array()?.as_slice(),
                self.b.hash_to_field_size()?.as_slice(),
                self.c.to_byte_array()?.as_slice(),
            ],
        )?;
        if H::ID != ::light_hasher::Poseidon::ID {
            result[0] = 0;
        }
        Ok(result)
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
        use ::light_hasher::to_byte_array::ToByteArray;
        use ::light_hasher::hash_to_field_size::HashToFieldSize;
        use ::light_hasher::Hasher;
        let mut result = ::light_hasher::Poseidon::hashv(
            &[self.a.to_byte_array()?.as_slice(), self.b.to_byte_array()?.as_slice()],
        )?;
        if ::light_hasher::Poseidon::ID != ::light_hasher::Poseidon::ID {
            result[0] = 0;
        }
        Ok(result)
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
        let mut result = H::hashv(
            &[self.a.to_byte_array()?.as_slice(), self.b.to_byte_array()?.as_slice()],
        )?;
        if H::ID != ::light_hasher::Poseidon::ID {
            result[0] = 0;
        }
        Ok(result)
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

    #[test]
    fn test_sha256_large_struct_with_pubkeys() {
        // Test that SHA256 can handle large structs with Pubkeys that would fail with Poseidon
        // This struct has 15 fields including Pubkeys without #[hash] attribute
        let input: ItemStruct = parse_quote! {
            struct LargeAccountSha {
                pub field1: u64,
                pub field2: u64,
                pub field3: u64,
                pub field4: u64,
                pub field5: u64,
                pub field6: u64,
                pub field7: u64,
                pub field8: u64,
                pub field9: u64,
                pub field10: u64,
                pub field11: u64,
                pub field12: u64,
                pub field13: u64,
                // Pubkeys without #[hash] attribute - this would fail with Poseidon
                pub owner: solana_program::pubkey::Pubkey,
                pub authority: solana_program::pubkey::Pubkey,
            }
        };

        // SHA256 should handle this fine
        let sha_result = derive_light_hasher_sha(input.clone());
        assert!(
            sha_result.is_ok(),
            "SHA256 should handle large structs with Pubkeys"
        );

        // Regular Poseidon hasher should fail due to field count (>12) and Pubkey without #[hash]
        let poseidon_result = derive_light_hasher(input);
        assert!(
            poseidon_result.is_err(),
            "Poseidon should fail with >12 fields and unhashed Pubkeys"
        );
    }

    #[test]
    fn test_sha256_vs_poseidon_hashing_behavior() {
        // Test a struct that both can handle to show the difference in hashing approach
        let input: ItemStruct = parse_quote! {
            struct TestAccount {
                pub data: [u8; 31],
                pub counter: u64,
            }
        };

        // Both should succeed
        let sha_result = derive_light_hasher_sha(input.clone());
        assert!(sha_result.is_ok());

        let poseidon_result = derive_light_hasher(input);
        assert!(poseidon_result.is_ok());

        // Verify SHA256 implementation serializes whole struct
        let sha_output = sha_result.unwrap();
        let sha_code = sha_output.to_string();

        // SHA256 should use try_to_vec() for whole struct serialization (account for spaces)
        assert!(
            sha_code.contains("try_to_vec") && sha_code.contains("BorshSerialize"),
            "SHA256 should serialize whole struct using try_to_vec. Actual code: {}",
            sha_code
        );
        assert!(
            sha_code.contains("result [0] = 0") || sha_code.contains("result[0] = 0"),
            "SHA256 should truncate first byte. Actual code: {}",
            sha_code
        );

        // Poseidon should use field-by-field hashing
        let poseidon_output = poseidon_result.unwrap();
        let poseidon_code = poseidon_output.to_string();

        assert!(
            poseidon_code.contains("to_byte_array") && poseidon_code.contains("as_slice"),
            "Poseidon should use field-by-field hashing with to_byte_array. Actual code: {}",
            poseidon_code
        );
    }

    #[test]
    fn test_sha256_no_field_limit() {
        // Test that SHA256 doesn't enforce the 12-field limit
        let input: ItemStruct = parse_quote! {
            struct ManyFieldsStruct {
                pub f1: u32, pub f2: u32, pub f3: u32, pub f4: u32,
                pub f5: u32, pub f6: u32, pub f7: u32, pub f8: u32,
                pub f9: u32, pub f10: u32, pub f11: u32, pub f12: u32,
                pub f13: u32, pub f14: u32, pub f15: u32, pub f16: u32,
                pub f17: u32, pub f18: u32, pub f19: u32, pub f20: u32,
            }
        };

        // SHA256 should handle 20 fields without issue
        let result = derive_light_hasher_sha(input);
        assert!(result.is_ok(), "SHA256 should handle any number of fields");
    }

    #[test]
    fn test_sha256_flatten_not_supported() {
        // Test that SHA256 rejects flatten attribute (not implemented)
        let input: ItemStruct = parse_quote! {
            struct FlattenStruct {
                #[flatten]
                pub inner: InnerStruct,
                pub data: u64,
            }
        };

        let result = derive_light_hasher_sha(input);
        assert!(result.is_err(), "SHA256 should reject flatten attribute");

        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("not supported in SHA256"),
            "Should mention SHA256 limitation"
        );
    }

    #[test]
    fn test_sha256_with_discriminator_integration() {
        // Test that shows LightHasherSha works with LightDiscriminatorSha for large structs
        // This would be impossible with regular Poseidon-based macros
        let input: ItemStruct = parse_quote! {
            struct LargeIntegratedAccount {
                pub field1: u64, pub field2: u64, pub field3: u64, pub field4: u64,
                pub field5: u64, pub field6: u64, pub field7: u64, pub field8: u64,
                pub field9: u64, pub field10: u64, pub field11: u64, pub field12: u64,
                pub field13: u64, pub field14: u64, pub field15: u64, pub field16: u64,
                pub field17: u64, pub field18: u64, pub field19: u64, pub field20: u64,
                // Pubkeys without #[hash] attribute
                pub owner: solana_program::pubkey::Pubkey,
                pub authority: solana_program::pubkey::Pubkey,
                pub delegate: solana_program::pubkey::Pubkey,
            }
        };

        // Both SHA256 hasher and discriminator should work
        let sha_hasher_result = derive_light_hasher_sha(input.clone());
        assert!(
            sha_hasher_result.is_ok(),
            "SHA256 hasher should work with large structs"
        );

        let sha_discriminator_result = crate::discriminator::discriminator(input.clone());
        assert!(
            sha_discriminator_result.is_ok(),
            "SHA256 discriminator should work with large structs"
        );

        // Regular Poseidon variants should fail
        let poseidon_hasher_result = derive_light_hasher(input);
        assert!(
            poseidon_hasher_result.is_err(),
            "Poseidon hasher should fail with large structs"
        );

        // Verify the generated code contains expected patterns
        let sha_hasher_code = sha_hasher_result.unwrap().to_string();
        assert!(
            sha_hasher_code.contains("try_to_vec"),
            "Should use serialization approach"
        );
        assert!(
            sha_hasher_code.contains("BorshSerialize"),
            "Should use Borsh serialization"
        );

        let sha_discriminator_code = sha_discriminator_result.unwrap().to_string();
        assert!(
            sha_discriminator_code.contains("LightDiscriminator"),
            "Should implement LightDiscriminator"
        );
        assert!(
            sha_discriminator_code.contains("LIGHT_DISCRIMINATOR"),
            "Should provide discriminator constant"
        );
    }

    #[test]
    fn test_complete_sha256_ecosystem_practical_example() {
        // Demonstrates a real-world scenario where SHA256 variants are essential
        // This struct would be impossible with Poseidon due to:
        // 1. >12 fields (23+ fields)
        // 2. Multiple Pubkeys without #[hash] attribute
        // 3. Large data structures
        let input: ItemStruct = parse_quote! {
            pub struct ComplexGameState {
                // Game metadata (13 fields)
                pub game_id: u64,
                pub round: u32,
                pub turn: u8,
                pub phase: u8,
                pub start_time: i64,
                pub end_time: i64,
                pub max_players: u8,
                pub current_players: u8,
                pub entry_fee: u64,
                pub prize_pool: u64,
                pub game_mode: u32,
                pub difficulty: u8,
                pub status: u8,

                // Player information (6 Pubkey fields - would require #[hash] with Poseidon)
                pub creator: solana_program::pubkey::Pubkey,
                pub winner: solana_program::pubkey::Pubkey,
                pub current_player: solana_program::pubkey::Pubkey,
                pub authority: solana_program::pubkey::Pubkey,
                pub treasury: solana_program::pubkey::Pubkey,
                pub program_id: solana_program::pubkey::Pubkey,

                // Game state data (4+ more fields)
                pub board_state: [u8; 64],    // Large array
                pub player_scores: [u32; 8],  // Array of scores
                pub moves_history: [u16; 32], // Move history
                pub special_flags: u32,

                // This gives us 23+ fields total - way beyond Poseidon's 12-field limit
            }
        };

        // SHA256 variants should handle this complex struct effortlessly
        let sha_hasher_result = derive_light_hasher_sha(input.clone());
        assert!(
            sha_hasher_result.is_ok(),
            "SHA256 hasher must handle complex real-world structs"
        );

        let sha_discriminator_result = crate::discriminator::discriminator(input.clone());
        assert!(
            sha_discriminator_result.is_ok(),
            "SHA256 discriminator must handle complex real-world structs"
        );

        // Poseidon would fail with this struct
        let poseidon_result = derive_light_hasher(input);
        assert!(
            poseidon_result.is_err(),
            "Poseidon cannot handle structs with >12 fields and unhashed Pubkeys"
        );

        // Verify SHA256 generates efficient serialization-based code
        let hasher_code = sha_hasher_result.unwrap().to_string();
        assert!(
            hasher_code.contains("try_to_vec"),
            "Should serialize entire struct efficiently"
        );
        assert!(
            hasher_code.contains("BorshSerialize"),
            "Should use Borsh for serialization"
        );
        assert!(
            hasher_code.contains("result [0] = 0") || hasher_code.contains("result[0] = 0"),
            "Should apply field size truncation. Actual code: {}",
            hasher_code
        );

        // Verify discriminator works correctly
        let discriminator_code = sha_discriminator_result.unwrap().to_string();
        assert!(
            discriminator_code.contains("ComplexGameState"),
            "Should target correct struct"
        );
        assert!(
            discriminator_code.contains("LIGHT_DISCRIMINATOR"),
            "Should provide discriminator constant"
        );
    }
}
