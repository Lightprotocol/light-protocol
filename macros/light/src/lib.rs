extern crate proc_macro;
use accounts::process_light_accounts;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput, ItemFn};
use traits::process_light_traits;
mod accounts;
mod pubkey;
mod traits;

/// Converts a base58 encoded public key into a byte array.
#[proc_macro]
pub fn pubkey(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as pubkey::PubkeyArgs);
    pubkey::pubkey(args)
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

/// Adds required fields to your anchor instruction for applying a zk-compressed
/// state transition.
///
/// ## Usage
/// Add `#[light_accounts]` to your struct. Ensure it's applied before Anchor's
/// `#[derive(Accounts)]` and Light's `#[derive(LightTraits)]`.
///
/// ## Example
/// Note: You will have to build your program IDL using Anchor's `idl-build`
/// feature, otherwise your IDL won't include these accounts.
/// ```ignore
/// #[light_accounts]
/// #[derive(Accounts)]
/// pub struct ExampleInstruction<'info> {
///     pub my_program: Program<'info, MyProgram>,
/// }
/// ```
/// This will expand to add the following fields to your struct:
/// - `light_system_program`:           Verifies and applies zk-compression
///                                     state transitions.
/// - `registered_program_pda`:         A light protocol PDA to authenticate
///                                     state tree updates.
/// - `noop_program`:                   The SPL noop program to write
///                                     compressed-account state as calldata to
///                                     the Solana ledger.
/// - `account_compression_authority`:  The authority for account compression
///                                     operations.
/// - `account_compression_program`:    Called by light_system_program. Updates
///                                     state trees.
/// - `system_program`:                 The Solana System program.
#[proc_macro_attribute]
pub fn light_accounts(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match process_light_accounts(input) {
        Ok(token_stream) => token_stream.into(),
        Err(err) => TokenStream::from(err.to_compile_error()),
    }
}

/// Implements traits on the given struct required for invoking The Light system
/// program via CPI.
///
/// ## Usage
///
/// Add `#[derive(LightTraits)]` to your struct which specifies the accounts
/// required for your Anchor program instruction. Specify the attributes
/// `self_program`, `fee_payer`, `authority`, and optionally `cpi_context` to
/// the relevant fields.
///
/// ### Attributes
///
/// - `self_program`:   Marks the field that represents the program invoking the
///                     light system program, i.e. your program. You need to
///                     list your program as part of the struct.
/// - `fee_payer`:      Marks the field that represents the account responsible
///                     for paying transaction fees. (Signer)
///
/// - `authority`:      User account, on behalf of which the program is creating
///                     the account.
/// - `cpi_context`:    TODO: explain cpi_context.
///
/// ### Required accounts (must specify exact name).
///
/// - `light_system_program`:           Light systemprogram. verifies & applies
///                                     compression state transitions.
/// - `registered_program_pda`:         Light Systemprogram PDA
/// - `noop_program`:                   SPL noop program
/// - `account_compression_authority`:  TODO: explain.
/// - `account_compression_program`:    Account Compression program.
/// - `system_program`:                 The Solana Systemprogram.
///
/// ### Example
/// ```ignore
/// #[derive(Accounts, LightTraits)]
/// pub struct ExampleInstruction<'info> {
///     #[self_program]
///     pub my_program: Program<'info, MyProgram>,
///     #[fee_payer]
///     pub payer: Signer<'info>,
///     #[authority]
///     pub user: AccountInfo<'info>,
///     #[cpi_context]
///     pub cpi_context_account: Account<'info, CpiContextAccount>,
///     pub light_system_program: Program<'info, LightSystemProgram>,
///     pub registered_program_pda: Account<'info, RegisteredProgram>,
///     pub noop_program: AccountInfo<'info>,
///     pub account_compression_authority: AccountInfo<'info>,
///     pub account_compression_program: Program<'info, AccountCompression>,
///     pub system_program: Program<'info, System>,
/// }
/// ```
#[proc_macro_derive(
    LightCompressedAccounts,
    attributes(self_program, fee_payer, authority, cpi_context)
)]
pub fn light_compressed_accounts(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match process_light_compressed_accounts(input) {
        Ok(token_stream) => token_stream.into(),
        Err(err) => TokenStream::from(err.to_compile_error()),
    }
}
