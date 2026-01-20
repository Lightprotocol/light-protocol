use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

use crate::hasher::field_processor::FieldProcessingContext;

pub(crate) fn generate_to_byte_array_impl_with_hasher(
    struct_name: &syn::Ident,
    generics: &syn::Generics,
    field_count: usize,
    context: &FieldProcessingContext,
    hasher: &TokenStream,
) -> Result<TokenStream> {
    let (impl_gen, type_gen, where_clause) = generics.split_for_impl();

    if field_count == 1 && !context.flatten_field_exists {
        let string = context.data_hasher_assignments[0].to_string();
        let alt_res = format!("Ok({})", string.as_str());
        // Removes clippy warning of unnessesary questionmark.
        let str = match string.strip_suffix("?") {
            Some(s) => s,
            None => &alt_res,
        };

        let content: TokenStream = str.parse().expect("Invalid generated code");
        Ok(quote! {
            impl #impl_gen ::light_hasher::to_byte_array::ToByteArray for #struct_name #type_gen #where_clause {
                const NUM_FIELDS: usize = 1;

                fn to_byte_array(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError> {
                    use ::light_hasher::to_byte_array::ToByteArray;
                    use ::light_hasher::hash_to_field_size::HashToFieldSize;
                    #content
                }
            }
        })
    } else {
        let data_hasher_assignments = &context.data_hasher_assignments;
        Ok(quote! {
            impl #impl_gen ::light_hasher::to_byte_array::ToByteArray for #struct_name #type_gen #where_clause {
                const NUM_FIELDS: usize = #field_count;

                fn to_byte_array(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError> {
                    use ::light_hasher::to_byte_array::ToByteArray;
                    use ::light_hasher::hash_to_field_size::HashToFieldSize;
                    use ::light_hasher::Hasher;
                    let mut result = #hasher::hashv(&[
                        #(#data_hasher_assignments.as_slice(),)*
                    ])?;

                    // Truncate field size for non-Poseidon hashers
                    if #hasher::ID != ::light_hasher::Poseidon::ID {
                        result[0] = 0;
                    }

                    Ok(result)
                }
            }
        })
    }
}

/// SHA256-specific ToByteArray implementation that serializes the whole struct
pub(crate) fn generate_to_byte_array_impl_sha(
    struct_name: &syn::Ident,
    generics: &syn::Generics,
    field_count: usize,
) -> Result<TokenStream> {
    let (impl_gen, type_gen, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_gen ::light_hasher::to_byte_array::ToByteArray for #struct_name #type_gen #where_clause {
            const NUM_FIELDS: usize = #field_count;

            fn to_byte_array(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError> {
                use borsh::BorshSerialize;
                use ::light_hasher::Hasher;

                // For SHA256, we can serialize the whole struct and hash it in one go
                let serialized = self.try_to_vec().map_err(|_| ::light_hasher::HasherError::BorshError)?;
                let mut result = ::light_hasher::Sha256::hash(&serialized)?;

                // Truncate field size for SHA256
                result[0] = 0;

                Ok(result)
            }
        }
    })
}
