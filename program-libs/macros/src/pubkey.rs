use bs58::decode;
use proc_macro2::TokenStream;
#[allow(unused)]
use quote::quote;
use syn::{parse::Parse, Error, LitStr, Result};

const PUBKEY_LEN: usize = 32;

pub(crate) struct PubkeyArgs {
    pub(crate) pubkey: LitStr,
}

impl Parse for PubkeyArgs {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(Self {
            pubkey: input.parse()?,
        })
    }
}

pub(crate) fn pubkey(args: PubkeyArgs) -> Result<TokenStream> {
    let v = decode(args.pubkey.value())
        .into_vec()
        .map_err(|_| Error::new(args.pubkey.span(), "Invalid base58 string"))?;
    let v_len = v.len();

    let arr: [u8; PUBKEY_LEN] = v.try_into().map_err(|_| {
        Error::new(
            args.pubkey.span(),
            format!(
                "Invalid size of decoded public key, expected 32, got {}",
                v_len,
            ),
        )
    })?;

    Ok(quote! {
            ::solana_program::pubkey::Pubkey::new_from_array([ #(#arr),* ])
    })
}

pub(crate) fn pubkey_array(args: PubkeyArgs) -> Result<TokenStream> {
    let v = decode(args.pubkey.value())
        .into_vec()
        .map_err(|_| Error::new(args.pubkey.span(), "Invalid base58 string"))?;
    let v_len = v.len();

    let arr: [u8; PUBKEY_LEN] = v.try_into().map_err(|_| {
        Error::new(
            args.pubkey.span(),
            format!(
                "Invalid size of decoded public key, expected 32, got {}",
                v_len,
            ),
        )
    })?;

    Ok(quote! {
        [ #(#arr),* ]
    })
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_pubkey() {
        let res = pubkey(parse_quote! { "cmtDvXumGCrqC1Age74AVPhSRVXJMd8PJS91L8KbNCK" });
        assert_eq!(
            res.unwrap().to_string(),
            ":: solana_program :: pubkey :: Pubkey :: new_from_array ([9u8 , 42u8 \
             , 19u8 , 238u8 , 149u8 , 196u8 , 28u8 , 186u8 , 8u8 , 166u8 , \
             127u8 , 90u8 , 198u8 , 126u8 , 141u8 , 247u8 , 225u8 , 218u8 , \
             17u8 , 98u8 , 94u8 , 29u8 , 100u8 , 19u8 , 127u8 , 143u8 , 79u8 , \
             35u8 , 131u8 , 3u8 , 127u8 , 20u8])",
        );
    }
}
