use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Field;

use super::utils::convert_to_zerocopy_type;

/// Generates the meta struct definition as a TokenStream
/// The `MUT` parameter determines if the struct should be generated for mutable access
pub fn generate_meta_struct<const MUT: bool>(
    z_struct_meta_name: &syn::Ident,
    meta_fields: &[&Field],
    hasher: bool,
) -> syn::Result<TokenStream> {
    let z_struct_meta_name = if MUT {
        format_ident!("{}Mut", z_struct_meta_name)
    } else {
        z_struct_meta_name.clone()
    };

    // Generate the meta struct fields with converted types
    let meta_fields_with_converted_types = meta_fields.iter().map(|field| {
        let field_name = &field.ident;
        let attributes = if hasher {
            field
                .attrs
                .iter()
                .map(|attr| {
                    quote! { #attr }
                })
                .collect::<Vec<_>>()
        } else {
            vec![quote! {}]
        };
        let field_type = convert_to_zerocopy_type(&field.ty);
        quote! {
            #(#attributes)*
            pub #field_name: #field_type
        }
    });
    let hasher = if hasher {
        quote! {
            , LightHasher
        }
    } else {
        quote! {}
    };

    // Return the complete meta struct definition
    let result = quote! {
        #[repr(C)]
        #[derive(Debug, PartialEq, ::light_zero_copy::KnownLayout, ::light_zero_copy::Immutable, ::light_zero_copy::Unaligned, ::light_zero_copy::FromBytes, ::light_zero_copy::IntoBytes #hasher)]
        pub struct #z_struct_meta_name {
            #(#meta_fields_with_converted_types,)*
        }
    };
    Ok(result)
}
