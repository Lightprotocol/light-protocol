use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemStruct};

mod expand;

/// Converts a base58 encoded public key into a byte array.
#[proc_macro]
pub fn pubkey(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as expand::PubkeyArgs);
    expand::pubkey(args)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn light_verifier_accounts(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as expand::LightVerifierAccountsArgs);
    let item_strct = item.clone();
    let strct = parse_macro_input!(item_strct as ItemStruct);

    expand::light_verifier_accounts(args, strct)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
