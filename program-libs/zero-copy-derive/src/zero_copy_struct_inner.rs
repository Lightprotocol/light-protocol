use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

/// Generates the ZeroCopyStructInner implementation as a TokenStream
pub fn generate_zero_copy_struct_inner<const MUT: bool>(
    name: &Ident,
    z_struct_name: &Ident,
) -> TokenStream {
    if MUT {
        quote! {
            // ZeroCopyStructInner implementation
            impl light_zero_copy::borsh_mut::ZeroCopyStructInnerMut for #name {
                type ZeroCopyInnerMut = #z_struct_name<'static>;
            }
        }
    } else {
        quote! {
            // ZeroCopyStructInner implementation
            impl light_zero_copy::borsh::ZeroCopyStructInner for #name {
                type ZeroCopyInner = #z_struct_name<'static>;
            }
        }
    }
}
