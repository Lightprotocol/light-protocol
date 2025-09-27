use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

use crate::hasher::field_processor::FieldProcessingContext;

pub(crate) fn generate_data_hasher_impl(
    struct_name: &syn::Ident,
    generics: &syn::Generics,
    context: &FieldProcessingContext,
) -> Result<TokenStream> {
    let (impl_gen, type_gen, where_clause) = generics.split_for_impl();

    // Need to create references for the quote! macro
    let hash_to_field_size_code = &context.hash_to_field_size_code;
    let data_hasher_assignments = &context.data_hasher_assignments;
    let flattened_fields_added = &context.flattened_fields_added;

    let hasher_impl = if context.flatten_field_exists {
        quote! {
            impl #impl_gen ::light_hasher::DataHasher for #struct_name #type_gen #where_clause {
                fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
                where
                    H: ::light_hasher::Hasher
                {
                    use ::light_hasher::DataHasher;
                    use ::light_hasher::Hasher;
                    use ::light_hasher::to_byte_array::ToByteArray;

                    #(#hash_to_field_size_code)*
                    let mut num_flattned_fields = 0;
                    let mut field_array = [[0u8; 32];  #(#flattened_fields_added)*];
                    let mut slices: [&[u8]; #(#flattened_fields_added)*] = [&[];  #(#flattened_fields_added)*];


                    for element in field_array.iter() {
                        slices[num_flattned_fields] = element.as_slice();
                    }

                    let mut result = H::hashv(slices.as_slice())?;

                    // Apply field size truncation for non-Poseidon hashers
                    if H::ID != ::light_hasher::Poseidon::ID {
                        result[0] = 0;
                    }

                    Ok(result)
                }
            }
        }
    } else {
        quote! {
            impl #impl_gen ::light_hasher::DataHasher for #struct_name #type_gen #where_clause {
                fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
                where
                    H: ::light_hasher::Hasher
                {
                    use ::light_hasher::DataHasher;
                    use ::light_hasher::Hasher;
                    use ::light_hasher::to_byte_array::ToByteArray;
                    #(#hash_to_field_size_code)*
                    #[cfg(debug_assertions)]
                   {
                       if std::env::var("RUST_BACKTRACE").is_ok() {
                            let debug_prints: Vec<[u8;32]> = vec![#(#data_hasher_assignments,)*];
                            println!("DataHasher::hash inputs {:?}", debug_prints);
                       }
                   }
                    let mut result = H::hashv(&[
                        #(#data_hasher_assignments.as_slice(),)*
                    ])?;

                    // Apply field size truncation for non-Poseidon hashers
                    if H::ID != ::light_hasher::Poseidon::ID {
                        result[0] = 0;
                    }

                    Ok(result)
                }
            }
        }
    };

    Ok(hasher_impl)
}

/// SHA256-specific DataHasher implementation that serializes the whole struct
pub(crate) fn generate_data_hasher_impl_sha(
    struct_name: &syn::Ident,
    generics: &syn::Generics,
) -> Result<TokenStream> {
    let (impl_gen, type_gen, where_clause) = generics.split_for_impl();

    let hasher_impl = quote! {
        impl #impl_gen ::light_hasher::DataHasher for #struct_name #type_gen #where_clause {
            fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
            where
                H: ::light_hasher::Hasher
            {
                use ::light_hasher::Hasher;
                use borsh::BorshSerialize;

                // Compile-time assertion that H must be SHA256 (ID = 1)
                use ::light_hasher::sha256::RequireSha256;
                let _ = <H as RequireSha256>::ASSERT;

                // For SHA256, we serialize the whole struct and hash it in one go
                let serialized = self.try_to_vec().map_err(|_| ::light_hasher::HasherError::BorshError)?;
                let mut result = H::hash(&serialized)?;
                // Truncate sha256 to 31 be bytes less than 254 bits bn254 field size.
                result[0] = 0;
                Ok(result)
            }
        }
    };

    Ok(hasher_impl)
}
