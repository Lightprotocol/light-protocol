extern crate proc_macro;
use accounts::{process_light_accounts, process_light_system_accounts};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput, ItemFn, ItemMod, ItemStruct};
use traits::process_light_traits;

mod account;
mod accounts;
mod discriminator;
mod hasher;
mod program;
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
/// Add `#[light_system_accounts]` to your struct. Ensure it's applied before Anchor's
/// `#[derive(Accounts)]` and Light's `#[derive(LightTraits)]`.
///
/// ## Example
/// Note: You will have to build your program IDL using Anchor's `idl-build`
/// feature, otherwise your IDL won't include these accounts.
/// ```ignore
/// #[light_system_accounts]
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
pub fn light_system_accounts(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);

    process_light_system_accounts(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn light_accounts(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);

    match process_light_accounts(input) {
        Ok(token_stream) => token_stream.into(),
        Err(err) => TokenStream::from(err.to_compile_error()),
    }
}

#[proc_macro_derive(LightAccounts, attributes(light_account))]
pub fn light_accounts_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    accounts::process_light_accounts_derive(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Implements traits on the given struct required for invoking The Light system
/// program via CPI.
///
/// ## Usage
/// Add `#[derive(LightTraits)]` to your struct which specifies the accounts
/// required for your Anchor program instruction. Specify the attributes
/// `self_program`, `fee_payer`, `authority`, and optionally `cpi_context` to
/// the relevant fields.
///
/// ### Attributes
/// - `self_program`:   Marks the field that represents the program invoking the
///                     light system program, i.e. your program. You need to
///                     list your program as part of the struct.
/// - `fee_payer`:      Marks the field that represents the account responsible
///                     for paying transaction fees. (Signer)
///
/// - `authority`:      TODO: explain authority.
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
    LightTraits,
    attributes(self_program, fee_payer, authority, cpi_context)
)]
pub fn light_traits_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match process_light_traits(input) {
        Ok(token_stream) => token_stream.into(),
        Err(err) => TokenStream::from(err.to_compile_error()),
    }
}

#[proc_macro_derive(LightDiscriminator)]
pub fn light_discriminator(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    discriminator::discriminator(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Makes the annotated struct hashable by implementing the following traits:
///
/// - [`AsByteVec`](light_hasher::bytes::AsByteVec), which makes the struct
///   convertable to a 2D byte vector.
/// - [`DataHasher`](light_hasher::DataHasher), which makes the struct hashable
///   with the `hash()` method, based on the byte inputs from `AsByteVec`
///   implementation.
///
/// This macro assumes that all the fields of the struct implement the
/// `AsByteVec` trait. The trait is implemented by default for the most of
/// standard Rust types (primitives, `String`, arrays and options carrying the
/// former). If there is a field of a type not implementing the trait, there
/// are two options:
///
/// 1. The most recommended one - annotating that type with the `light_hasher`
///    macro as well.
/// 2. Manually implementing the `AsByteVec` trait.
///
/// # Attributes
///
/// - `skip` - skips the given field, it doesn't get included neither in
///   `AsByteVec` nor `DataHasher` implementation.
/// - `truncate` - makes sure that the byte value does not exceed the BN254
///   prime field modulus, by hashing it (with Keccak) and truncating it to 31
///   bytes. It's generally a good idea to use it on any field which is
///   expected to output more than 31 bytes.
///
/// # Examples
///
/// Compressed account with only primitive types as fields:
///
/// ```ignore
/// #[derive(LightHasher)]
/// pub struct MyCompressedAccount {
///     a: i64,
///     b: Option<u64>,
/// }
/// ```
///
/// Compressed account with fields which might exceed the BN254 prime field:
///
/// ```ignore
/// #[derive(LightHasher)]
/// pub struct MyCompressedAccount {
///     a: i64
///     b: Option<u64>,
///     #[truncate]
///     c: [u8; 32],
///     #[truncate]
///     d: String,
/// }
/// ```
///
/// Compressed account with fields we want to skip:
///
/// ```ignore
/// #[derive(LightHasher)]
/// pub struct MyCompressedAccount {
///     a: i64
///     b: Option<u64>,
///     #[skip]
///     c: [u8; 32],
/// }
/// ```
///
/// Compressed account with a nested struct:
///
/// ```ignore
/// #[derive(LightHasher)]
/// pub struct MyCompressedAccount {
///     a: i64
///     b: Option<u64>,
///     c: MyStruct,
/// }
///
/// #[derive(LightHasher)]
/// pub struct MyStruct {
///     a: i32
///     b: u32,
/// }
/// ```
///
/// Compressed account with a type with a custom `AsByteVec` implementation:
///
/// ```ignore
/// #[derive(LightHasher)]
/// pub struct MyCompressedAccount {
///     a: i64
///     b: Option<u64>,
///     c: RData,
/// }
///
/// pub enum RData {
///     A(Ipv4Addr),
///     AAAA(Ipv6Addr),
///     CName(String),
/// }
///
/// impl AsByteVec for RData {
///     fn as_byte_vec(&self) -> Vec<Vec<u8>> {
///         match self {
///             Self::A(ipv4_addr) => vec![ipv4_addr.octets().to_vec()],
///             Self::AAAA(ipv6_addr) => vec![ipv6_addr.octets().to_vec()],
///             Self::CName(cname) => cname.as_byte_vec(),
///         }
///     }
/// }
/// ```
#[proc_macro_derive(LightHasher, attributes(skip, truncate))]
pub fn light_hasher(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    hasher::hasher(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn light_account(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    account::account(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn light_program(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemMod);
    program::program(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
