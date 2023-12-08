use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, FnArg, ItemFn, ItemStruct, Receiver, Signature};

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
    #[allow(clippy::redundant_clone)]
    let item_strct = item.clone();
    let strct = parse_macro_input!(item_strct as ItemStruct);

    expand::light_verifier_accounts(args, strct)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn light_public_transaction(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as expand::LightVerifierAccountsArgs);
    #[allow(clippy::redundant_clone)]
    let item_strct = item.clone();
    let strct = parse_macro_input!(item_strct as ItemStruct);

    expand::light_public_transaction_accounts(args, strct)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn heap_neutral(_: TokenStream, input: TokenStream) -> TokenStream {
    let mut function = parse_macro_input!(input as ItemFn);

    // Check if the function signature uses `&self` and not `&mut self`
    if !is_immutable_self(&function.sig) {
        return syn::Error::new_spanned(
            &function.sig,
            "This macro requires the function to use `&self` and not `&mut self`",
        )
        .to_compile_error()
        .into();
    }

    // Insert memory management code at the beginning of the function
    let init_code: syn::Stmt = parse_quote! {
        #[cfg(target_os = "solana")]
        let pos = custom_heap::get_heap_pos();
    };
    function.block.stmts.insert(0, init_code);

    // Insert memory management code at the end of the function
    let cleanup_code: syn::Stmt = parse_quote! {
        #[cfg(target_os = "solana")]
        custom_heap::free_heap(pos);
    };
    let len = function.block.stmts.len();
    function.block.stmts.insert(len - 1, cleanup_code);

    TokenStream::from(quote! { #function })
}

fn is_immutable_self(signature: &Signature) -> bool {
    signature.inputs.iter().any(|arg| {
        matches!(
            arg,
            FnArg::Receiver(Receiver {
                reference: Some(_),
                mutability: None,
                ..
            })
        )
    })
}
