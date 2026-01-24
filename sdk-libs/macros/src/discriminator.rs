use light_hasher::{Hasher, Sha256};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemStruct, Result};

/// Light discriminator: SHA256("{name}")[0..8]
/// Implements LightDiscriminator trait
pub(crate) fn light_discriminator(input: ItemStruct) -> Result<TokenStream> {
    let account_name = &input.ident;
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

/// Anchor discriminator: SHA256("account:{name}")[0..8]
/// Implements the SAME LightDiscriminator trait, just with different hash input
pub(crate) fn anchor_discriminator(input: ItemStruct) -> Result<TokenStream> {
    let account_name = &input.ident;
    let hash_input = format!("account:{}", account_name);

    let (impl_gen, type_gen, where_clause) = input.generics.split_for_impl();

    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&Sha256::hash(hash_input.as_bytes()).unwrap()[..8]);
    let discriminator: proc_macro2::TokenStream = format!("{discriminator:?}").parse().unwrap();

    // Same trait, different value
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
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_light_discriminator() {
        let input: ItemStruct = parse_quote! {
            struct MyAccount {
                a: u32,
                b: i32,
                c: u64,
                d: i64,
            }
        };

        let output = light_discriminator(input).unwrap();
        let output = output.to_string();

        assert!(output.contains("impl LightDiscriminator for MyAccount"));
        // SHA256("MyAccount")[0..8]
        assert!(output.contains("[181 , 255 , 112 , 42 , 17 , 188 , 66 , 199]"));
    }

    #[test]
    fn test_anchor_discriminator() {
        let input: ItemStruct = parse_quote! {
            struct MyAccount {
                a: u32,
                b: i32,
                c: u64,
                d: i64,
            }
        };

        let output = anchor_discriminator(input).unwrap();
        let output = output.to_string();

        assert!(output.contains("impl LightDiscriminator for MyAccount"));
        // SHA256("account:MyAccount")[0..8] = f6 1c 06 57 fb 2d 32 2a
        assert!(output.contains("[246 , 28 , 6 , 87 , 251 , 45 , 50 , 42]"));
    }
}
