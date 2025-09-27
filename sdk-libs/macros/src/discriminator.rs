use light_hasher::{Hasher, Sha256};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemStruct, Result};

pub(crate) fn discriminator(input: ItemStruct) -> Result<TokenStream> {
    let account_name = &input.ident;
    // When anchor-discriminator-compat feature is enabled, use "account:" prefix like Anchor does
    #[cfg(feature = "anchor-discriminator")]
    let hash_input = format!("account:{}", account_name);

    #[cfg(not(feature = "anchor-discriminator"))]
    let hash_input = account_name.to_string();

    let (impl_gen, type_gen, where_clause) = input.generics.split_for_impl();

    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&Sha256::hash(hash_input.as_bytes()).unwrap()[..8]);
    let discriminator: proc_macro2::TokenStream = format!("{discriminator:?}").parse().unwrap();

    Ok(quote! {
        impl #impl_gen LightDiscriminator for #account_name #type_gen #where_clause {
            const LIGHT_DISCRIMINATOR: [u8; 8] = #discriminator;
            const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;

            fn discriminator() -> [u8; 8] {
                Self::LIGHT_DISCRIMINATOR
            }
        }
    })
}

#[cfg(test)]
mod tests {

    #[cfg(not(feature = "anchor-discriminator"))]
    #[test]
    fn test_discriminator() {
        use syn::parse_quote;

        use super::*;

        let input: ItemStruct = parse_quote! {
            struct MyAccount {
                a: u32,
                b: i32,
                c: u64,
                d: i64,
            }
        };

        let output = discriminator(input).unwrap();
        let output = output.to_string();

        assert!(output.contains("impl LightDiscriminator for MyAccount"));
        assert!(output.contains("[181 , 255 , 112 , 42 , 17 , 188 , 66 , 199]"));
    }
}
