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

                    H::hashv(slices.as_slice())
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
                    H::hashv(&[
                        #(#data_hasher_assignments.as_slice(),)*
                    ])
                }
            }
        }
    };

    Ok(hasher_impl)
}
