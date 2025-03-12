use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Field;

use crate::utils::convert_to_zerocopy_type;

/// Generates the meta struct definition as a TokenStream
/// The `MUT` parameter determines if the struct should be generated for mutable access
pub fn generate_meta_struct<const MUT: bool>(
    z_struct_meta_name: &syn::Ident,
    meta_fields: &[&Field],
) -> TokenStream {
    let mut z_struct_meta_name = z_struct_meta_name.clone();
    if MUT {
        z_struct_meta_name = format_ident!("{}Mut", z_struct_meta_name);
    }

    // Generate the meta struct fields with converted types
    let meta_fields_with_converted_types = meta_fields.iter().map(|field| {
        let field_name = &field.ident;
        let field_type = convert_to_zerocopy_type(&field.ty);
        quote! {
            pub #field_name: #field_type
        }
    });

    // Return the complete meta struct definition
    quote! {
        #[repr(C)]
        #[derive(Debug, PartialEq, zerocopy::KnownLayout, zerocopy::Immutable, zerocopy::Unaligned, zerocopy::FromBytes, zerocopy::IntoBytes)]
        pub struct #z_struct_meta_name {
            #(#meta_fields_with_converted_types,)*
        }
    }
}
