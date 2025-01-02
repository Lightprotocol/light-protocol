use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemStruct, Result};

pub(crate) fn account(input: ItemStruct) -> Result<TokenStream> {
    Ok(quote! {
        #[derive(
            ::anchor_lang::AnchorDeserialize,
            ::anchor_lang::AnchorSerialize,
            ::light_sdk::LightDiscriminator,
            ::light_sdk::LightHasher,
        )]
        #input
    })
}
