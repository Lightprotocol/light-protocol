use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

use crate::hasher::field_processor::FieldProcessingContext;

pub(crate) fn generate_to_byte_array_impl(
    struct_name: &syn::Ident,
    generics: &syn::Generics,
    field_count: usize,
    context: &FieldProcessingContext,
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
        let field_assignment: TokenStream = syn::parse_str(str)?;

        // Create a token stream with the field_assignment and the import code
        let mut hash_imports = proc_macro2::TokenStream::new();
        for code in &context.hash_to_field_size_code {
            hash_imports.extend(code.clone());
        }

        Ok(quote! {
            impl #impl_gen ::light_hasher::to_byte_array::ToByteArray for #struct_name #type_gen #where_clause {
                const NUM_FIELDS: usize = #field_count;

                fn to_byte_array(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError> {
                    #hash_imports
                    #field_assignment
                }
            }
        })
    } else {
        Ok(quote! {
            impl #impl_gen ::light_hasher::to_byte_array::ToByteArray for #struct_name #type_gen #where_clause {
                const NUM_FIELDS: usize = #field_count;

                fn to_byte_array(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError> {
                    ::light_hasher::DataHasher::hash::<::light_hasher::Poseidon>(self)
                }

            }
        })
    }
}
