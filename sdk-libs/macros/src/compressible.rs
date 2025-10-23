use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

// Parse a comma-separated list of identifiers
// #[derive(Clone)]
// #[allow(dead_code)]
// enum CompressibleType {
//     Regular(Ident),
// }

// struct CompressibleTypeList {
//     types: Punctuated<CompressibleType, Token![,]>,
// }

// impl Parse for CompressibleType {
//     fn parse(input: ParseStream) -> Result<Self> {
//         let ident: Ident = input.parse()?;
//         Ok(CompressibleType::Regular(ident))
//     }
// }

// impl Parse for CompressibleTypeList {
//     fn parse(input: ParseStream) -> Result<Self> {
//         Ok(CompressibleTypeList {
//             types: Punctuated::parse_terminated(input)?,
//         })
//     }
// }

/// Generates HasCompressionInfo trait implementation for a struct with compression_info field
pub fn derive_has_compression_info(input: syn::ItemStruct) -> Result<TokenStream> {
    let struct_name = input.ident.clone();

    // Find the compression_info field
    let compression_info_field = match &input.fields {
        syn::Fields::Named(fields) => fields.named.iter().find(|field| {
            field
                .ident
                .as_ref()
                .map(|ident| ident == "compression_info")
                .unwrap_or(false)
        }),
        _ => {
            return Err(syn::Error::new_spanned(
                &struct_name,
                "HasCompressionInfo can only be derived for structs with named fields",
            ))
        }
    };

    let _compression_info_field = compression_info_field.ok_or_else(|| {
        syn::Error::new_spanned(
            &struct_name,
            "HasCompressionInfo requires a field named 'compression_info' of type Option<CompressionInfo>"
        )
    })?;

    // Validate that the field is Option<CompressionInfo>. For now, we'll assume
    // it's correct and let the compiler catch type errors
    let has_compression_info_impl = quote! {
        impl light_sdk::compressible::HasCompressionInfo for #struct_name {
            fn compression_info(&self) -> &light_sdk::compressible::CompressionInfo {
                self.compression_info
                    .as_ref()
                    .expect("CompressionInfo must be Some on-chain")
            }

            fn compression_info_mut(&mut self) -> &mut light_sdk::compressible::CompressionInfo {
                self.compression_info
                    .as_mut()
                    .expect("CompressionInfo must be Some on-chain")
            }

            fn compression_info_mut_opt(&mut self) -> &mut Option<light_sdk::compressible::CompressionInfo> {
                &mut self.compression_info
            }

            fn set_compression_info_none(&mut self) {
                self.compression_info = None;
            }
        }
    };

    Ok(has_compression_info_impl)
}
