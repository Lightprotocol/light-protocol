extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, ItemFn};

mod cpi_signer;
mod pubkey;

/// Converts a base58 encoded public key into a byte array.
#[proc_macro]
pub fn pubkey(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as pubkey::PubkeyArgs);
    pubkey::pubkey(args)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Converts a base58 encoded public key into a raw byte array [u8; 32].
#[proc_macro]
pub fn pubkey_array(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as pubkey::PubkeyArgs);
    pubkey::pubkey_array(args)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
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
        light_heap::GLOBAL_ALLOCATOR.free_heap(pos)?;
    };
    let len = function.block.stmts.len();
    function.block.stmts.insert(len - 1, log_post);
    function.block.stmts.insert(len - 1, cleanup_code);
    TokenStream::from(quote! { #function })
}

/// No-op derive macro that does nothing.
/// Used as a placeholder for serialization derives when not needed.
#[proc_macro_derive(Noop)]
pub fn derive_noop(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}

/// Derives a Light Protocol CPI signer PDA at compile time
///
/// This macro computes the CPI signer PDA using the "cpi_authority" seed
/// for the given program ID. Uses `solana_pubkey` with `solana` feature,
/// otherwise uses `pinocchio` (default).
///
/// ## Usage
///
/// ```rust
/// # use light_macros::derive_light_cpi_signer_pda;
/// // In a Solana program
/// let (cpi_signer, bump) = derive_light_cpi_signer_pda!("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7");
/// ```
///
/// Returns a tuple `([u8; 32], u8)` containing the PDA address and bump seed.
#[proc_macro]
pub fn derive_light_cpi_signer_pda(input: TokenStream) -> TokenStream {
    cpi_signer::derive_light_cpi_signer_pda(input)
}

/// Derives a complete Light Protocol CPI configuration at runtime
///
/// This macro computes the program ID, CPI signer PDA, and bump seed
/// for the given program ID. Uses `solana_pubkey` with `solana` feature,
/// otherwise uses `pinocchio` (default).
///
/// ## Usage
///
/// ```rust
/// # use light_macros::derive_light_cpi_signer;
/// # struct CpiSigner { program_id: [u8; 32], cpi_signer: [u8; 32], bump: u8 }
/// // In a Solana program
/// const LIGHT_CPI_SIGNER: CpiSigner = derive_light_cpi_signer!("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7");
/// ```
///
/// Returns a `CpiSigner` struct (must be in scope) containing:
/// - `program_id`: Program ID bytes
/// - `cpi_signer`: CPI signer PDA address
/// - `bump`: Bump seed
#[proc_macro]
pub fn derive_light_cpi_signer(input: TokenStream) -> TokenStream {
    cpi_signer::derive_light_cpi_signer(input)
}
