use light_hasher::{Hasher, Sha256};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemStruct, Result};

pub(crate) fn discriminator(input: ItemStruct) -> Result<TokenStream> {
    let account_name = &input.ident;

    let (impl_gen, type_gen, where_clause) = input.generics.split_for_impl();

    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&Sha256::hash(account_name.to_string().as_bytes()).unwrap()[..8]);
    let discriminator: proc_macro2::TokenStream = format!("{discriminator:?}").parse().unwrap();

    Ok(quote! {
        impl #impl_gen light_hasher::Discriminator for #account_name #type_gen #where_clause {
            const DISCRIMINATOR: [u8; 8] = #discriminator;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use syn::parse_quote;

    #[test]
    fn test_discriminator() {
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

        assert!(output.contains("impl light_hasher :: Discriminator for MyAccount"));
    }
}
