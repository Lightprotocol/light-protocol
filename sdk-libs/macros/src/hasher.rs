use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_str, Error, Fields, ItemStruct, Result};

/// - ToByteArray:
///     1. ToByteArray -> [u8;32]
///     2. ToByteArrays -> [[u8;32]; NUM_FIELDS]
///     3. const NumFields -> usize (can be used to get ToByteArrays)
/// - DataHasher Hash -> [u8;32]
///
/// - Attribute Macros:
///     1. hash
///         hash to bn254 field size (less than 254 bit), hash with keccak256 and truncate to 31 bytes
///     2. skip
///         ignore field
///     3. flatten
///         flatten nested struct or vector
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

    let field_count = fields.named.len();
    if field_count >= 13 {
        unimplemented!("Structs with more than 13 fields are not supported.");
    }
    let mut code = Vec::new();
    let mut added_flattned_field = false;
    let mut to_byte_arrays_fields = Vec::new();
    let mut truncate_set = false;
    let flatten_field_exists = fields.named.iter().any(|field| {
        field
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("flatten"))
    });

    let mut flattned_fields_added = Vec::new();
    let mut truncate_code = Vec::new();

    // Process each field
    let mut field_assignments = Vec::new();
    fields.named.iter().enumerate().for_each(|(i, field)| {
        let field_name = &field.ident;
        let hash_to_field_size = field
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("hash"));
        let skip = field.attrs.iter().any(|attr| attr.path().is_ident("skip"));
        let flatten = field
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("flatten"));

        // HashToFieldSize:
        // 1. General case: self.#field_name.hash_to_field_size()?
        // 2. Vec<u8> -> hashv_to_bn254_field_size_le(&[self.#field_name.as_slice()])
        // 3. Option<Vec<u8>> -> if let Some(#field_name) = self.#field_name { hashv_to_bn254_field_size_le(&[self.#field_name.as_slice()]) } else { [0u8;32] }
        if hash_to_field_size {
            if !truncate_set {
                truncate_code.push(quote! {
                    use ::light_hasher::hash_to_field_size::HashToFieldSize;
                });
                truncate_set = true;
            }
            if field.ty.to_token_stream().to_string() == "Vec < u8 >"{
                to_byte_arrays_fields.push(quote! {
                    arrays[#i ] = ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(self.#field_name.as_slice())?;
                });
                if flatten_field_exists {
                    field_assignments.push(quote! {
                        field_array[#i + num_flattned_fields ] = ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(self.#field_name.as_slice())?.as_slice();
                        slices[#i + num_flattned_fields ] = field_array[#i +  num_flattned_fields].as_slice();
                    });
                } else {
                    field_assignments.push(quote! {
                        ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(self.#field_name.as_slice())?
                    });
                }
            } else if field.ty.to_token_stream().to_string().starts_with("Option < Vec < u8 > >") {
                // HashToFieldSize the inner type if something is an option.
                to_byte_arrays_fields.push(quote! {
                    arrays[#i ] = if let Some(#field_name) = &self.#field_name {
                        ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(self.#field_name.as_slice())?
                    } else {
                        [0u8;32]
                    };
                });
                if flatten_field_exists {
                    field_assignments.push(quote! {
                        field_array[#i + num_flattned_fields ] = &self.#field_name {
                            ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(self.#field_name.as_slice())?
                        } else {
                            [0u8;32]
                        };
                        slices[#i + num_flattned_fields ] = field_array[#i +  num_flattned_fields].as_slice();
                    });
                } else {
                    field_assignments.push(quote! {
                        {
                            if let Some(#field_name) = &self.#field_name {
                                    ::light_hasher::hash_to_field_size::hashv_to_bn254_field_size_le(#field_name.as_slice()).as_slice()
                            } else {
                                    [0u8;32]
                            }
                        }
                    });
                }
            } else if field.ty.to_token_stream().to_string().starts_with("Option < ") {
                // TODO: consider is it necessary to hash Poseidon(self.hash_to_field_size) if is some ?
                // - if we hash and truncate already it is not necessary
                to_byte_arrays_fields.push(quote! {
                    arrays[#i ] = if let Some(#field_name) = &self.#field_name {
                        #field_name.hash_to_field_size()?
                    } else {
                        [0u8;32]
                    };
                });
                if flatten_field_exists {
                    field_assignments.push(quote! {
                        field_array[#i + num_flattned_fields ] = if let Some(#field_name) = &self.#field_name {
                            #field_name.hash_to_field_size()?
                        } else {
                            [0u8;32]
                        };
                        slices[#i + num_flattned_fields ] = field_array[#i +  num_flattned_fields].as_slice();
                    });
                } else {
                    field_assignments.push(quote! {
                        {
                            if let Some(#field_name) = &self.#field_name {
                                #field_name.hash_to_field_size()?
                            } else {
                                [0u8;32]
                            }
                        }
                    });
                }
            } else {
                to_byte_arrays_fields.push(quote! {
                    arrays[#i ] = self.#field_name.hash_to_field_size()?;
                });
                if flatten_field_exists {
                    field_assignments.push(quote! {
                        field_array[#i + num_flattned_fields ] = self.#field_name.hash_to_field_size()?;
                    });
                } else {
                    field_assignments.push(quote! {
                        self.#field_name.hash_to_field_size()?
                    });
                }
            }
        } else if skip {
        }
        else if flatten {
            let field_type = &field.ty;
            if !added_flattned_field {
                added_flattned_field = true;
                flattned_fields_added.push(quote! {
                    #field_type::NUM_FIELDS as usize
                });
            }else {
                flattned_fields_added.push(quote! {
                    + #field_type::NUM_FIELDS as usize
                });
            }
            code.push(quote! {
                {
                    for (j, element) in <#field_type as ::light_hasher::to_byte_array::ToByteArray>::to_byte_arrays::<{#field_type::NUM_FIELDS}>(&self.#field_name)?.iter().enumerate() {
                        field_array[#i + j + num_flattned_fields ] = *element;
                        num_flattned_fields +=1;
                    }
                }
            });
        } else {
            to_byte_arrays_fields.push(quote! {
                arrays[#i ] =  self.#field_name.to_byte_array()?;
            });
            if flatten_field_exists {
                field_assignments.push(quote! {
                    field_array[#i + num_flattned_fields ] = self.#field_name.to_byte_array()?;
                });
            } else {
                field_assignments.push(quote! {
                    self.#field_name.to_byte_array()?
                });
            }
        }
    });

    let hasher_impl = if flatten_field_exists {
        // Insert in front of all other flattening code
        // Do it here so that we have collected all flattned_fields_added.
        code.insert(
            0,
            quote! {
                    let mut num_flattned_fields = 0;
                    let mut field_array = [[0u8; 32];  #(#flattned_fields_added)*];
                    let mut slices: [&[u8]; #(#flattned_fields_added)*] = [&[];  #(#flattned_fields_added)*];
            },
        );
        code.push(quote! {
            for element in field_array.iter() {
                slices[num_flattned_fields] = element.as_slice();
            }
        });
        quote! {
        impl #impl_gen ::light_hasher::DataHasher for #struct_name #type_gen #where_clause {
            fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
            where
                H: ::light_hasher::Hasher
            {
                use ::light_hasher::DataHasher;
                use ::light_hasher::Hasher;
                use ::light_hasher::to_byte_array::ToByteArray;

                #(#truncate_code)*
                #(#code)*
                H::hashv(slices.as_slice())
            }
        }}
    } else {
        quote! {  impl #impl_gen ::light_hasher::DataHasher for #struct_name #type_gen #where_clause {
            fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
            where
                H: ::light_hasher::Hasher
            {
                use ::light_hasher::DataHasher;
                use ::light_hasher::Hasher;
                use ::light_hasher::to_byte_array::ToByteArray;
                #(#truncate_code)*
                #(#code)*
                H::hashv(&[
                    #(#field_assignments.as_slice(),)*
                ])
            }
        }}
    };

    let to_byte_array = if field_count == 1 && !flatten_field_exists {
        let string = field_assignments[0].to_string();
        let alt_res = format!("Ok({})", string.as_str());
        // Removes clippy warning of ununeeded question mark.
        let str = match string.strip_suffix("?") {
            Some(s) => s,
            None => &alt_res,
        };
        let field_assingment: TokenStream = parse_str(str).unwrap();
        // let first_field_name = first_field_name.expect("Expected first field name");
        quote! {
            #(#truncate_code)*
            #field_assingment
        }
    } else {
        quote! {
            ::light_hasher::DataHasher::hash::<::light_hasher::Poseidon>(self)
        }
    };

    Ok(quote! {
        impl #impl_gen ::light_hasher::to_byte_array::ToByteArray for #struct_name #type_gen #where_clause {
            const NUM_FIELDS: usize = #field_count;

            fn to_byte_array(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError> {
                #to_byte_array
            }

            fn to_byte_arrays<const NUM_FIELDS: usize>(&self) -> ::std::result::Result<[[u8; 32]; NUM_FIELDS], ::light_hasher::HasherError> {
                if Self::NUM_FIELDS != NUM_FIELDS {
                    return Err(::light_hasher::HasherError::InvalidNumFields);
                }
                #(#truncate_code)*
                let mut arrays = [[0u8; 32]; NUM_FIELDS];

                #(#to_byte_arrays_fields)*
                Ok(arrays)
            }
        }

        #hasher_impl
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
        assert!(formatted_output
            .contains("impl ::light_hasher::to_byte_array::ToByteArray for MyAccount"));
        assert!(formatted_output.contains("const NUM_FIELDS: usize = 4usize"));
        assert!(formatted_output.contains("fn to_byte_array"));
        assert!(formatted_output.contains("fn to_byte_arrays"));
        assert!(formatted_output.contains("arrays[0usize] = self.a.to_byte_array()?"));
        assert!(formatted_output.contains("arrays[1usize] = self.b.to_byte_array()?"));
        assert!(formatted_output.contains("arrays[2usize] = self.c.to_byte_array()?"));
        assert!(formatted_output.contains("arrays[3usize] = self.d.to_byte_array()?"));
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
        assert!(formatted_output.contains("const NUM_FIELDS: usize"));
        assert!(formatted_output.contains("2usize"));
        assert!(formatted_output.contains("fn to_byte_arrays"));
        assert!(formatted_output.contains("arrays[0usize]"));
        assert!(formatted_output.contains("arrays[1usize]"));
    }

    #[test]
    fn test_truncate_option() {
        let input: ItemStruct = parse_quote! {
            struct TruncateOptionStruct {
                #[hash]
                a: Option<String>,
            }
        };

        let output = hasher(input).unwrap();
        let formatted_output = unparse(&syn::parse2(output).unwrap());

        assert!(formatted_output.contains("const NUM_FIELDS: usize"));
        assert!(formatted_output.contains("1usize"));
        assert!(
            formatted_output.contains("use ::light_hasher::hash_to_field_size::HashToFieldSize")
        );
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

        let output = hasher(input).unwrap();
        let formatted_output = unparse(&syn::parse2(output).unwrap());

        assert!(formatted_output.contains("const NUM_FIELDS: usize"));
        assert!(formatted_output.contains("3usize"));
        assert!(formatted_output.contains("arrays[0usize] = self.a.to_byte_array()?"));
        assert!(formatted_output.contains("arrays[1usize] = self.b.hash_to_field_size()?"));
        assert!(formatted_output.contains("arrays[2usize]"));
    }

    #[test]
    fn test_nested_struct() {
        let input: ItemStruct = parse_quote! {
            struct OuterStruct {
                a: u32,
                b: InnerStruct,
            }
        };

        let output = hasher(input).unwrap();
        let formatted_output = unparse(&syn::parse2(output).unwrap());

        assert!(formatted_output.contains("const NUM_FIELDS: usize = 2usize"));
        assert!(formatted_output.contains("arrays[0usize] = self.a.to_byte_array()?"));
        assert!(formatted_output.contains("arrays[1usize] = self.b.to_byte_array()?"));
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
        assert!(hasher(input).is_ok());

        // In the new implementation, we don't have the nested attribute anymore
        // and there shouldn't be an error when using truncate with Option<u32>
        let input: ItemStruct = parse_quote! {
            struct ValidStruct {
                #[hash]
                a: Option<u32>,
            }
        };
        assert!(hasher(input).is_ok());
    }
}
