use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, ItemFn, ItemStruct};

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

#[proc_macro]
pub fn custom_heap(_input: TokenStream) -> TokenStream {
    TokenStream::from(quote! {
        #[global_allocator]
        pub static GLOBAL_ALLOCATOR: light_heap::BumpAllocator = light_heap::BumpAllocator {
            start: anchor_lang::solana_program::entrypoint::HEAP_START_ADDRESS as usize,
            len: anchor_lang::solana_program::entrypoint::HEAP_LENGTH,
        };
    })
}

#[proc_macro_attribute]
pub fn heap_neutral(_: TokenStream, input: TokenStream) -> TokenStream {
    let mut function = parse_macro_input!(input as ItemFn);

    // Insert memory management code at the beginning of the function
    let init_code: syn::Stmt = parse_quote! {
        #[cfg(target_os = "solana")]
        let pos = light_heap::GLOBAL_ALLOCATOR.get_heap_pos();
    };
    let msg = format!("pre: {}", function.sig.ident);
    let log_pre: syn::Stmt = parse_quote! {
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        light_heap::GLOBAL_ALLOCATOR.log_total_heap(#msg);
    };
    function.block.stmts.insert(0, init_code);
    function.block.stmts.insert(1, log_pre);

    // Insert memory management code at the end of the function
    let msg = format!("post: {}", function.sig.ident);
    let log_post: syn::Stmt = parse_quote! {
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        light_heap::GLOBAL_ALLOCATOR.log_total_heap(#msg);
    };
    let cleanup_code: syn::Stmt = parse_quote! {
        #[cfg(target_os = "solana")]
        light_heap::GLOBAL_ALLOCATOR.free_heap(pos);
    };
    let len = function.block.stmts.len();
    function.block.stmts.insert(len - 1, log_post);
    function.block.stmts.insert(len - 1, cleanup_code);

    TokenStream::from(quote! { #function })
}
