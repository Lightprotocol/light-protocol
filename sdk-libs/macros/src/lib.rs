extern crate proc_macro;
use accounts::{process_light_accounts, process_light_system_accounts};
use hasher::{derive_light_hasher, derive_light_hasher_sha};
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, ItemMod, ItemStruct};
use traits::process_light_traits;

mod account;
mod accounts;
mod compress_as;
mod compressible;
mod compressible_derive;
mod discriminator;
mod hasher;
mod program;
mod traits;

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
///   state transitions.
/// - `registered_program_pda`:         A light protocol PDA to authenticate
///   state tree updates.
/// - `noop_program`:                   The SPL noop program to write
///   compressed-account state as calldata to
///   the Solana ledger.
/// - `account_compression_authority`:  The authority for account compression
///   operations.
/// - `account_compression_program`:    Called by light_system_program. Updates
///   state trees.
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
///   light system program, i.e. your program. You need to
///   list your program as part of the struct.
/// - `fee_payer`:      Marks the field that represents the account responsible
///   for paying transaction fees. (Signer)
///
/// - `authority`:      TODO: explain authority.
/// - `cpi_context`:    TODO: explain cpi_context.
///
/// ### Required accounts (must specify exact name).
///
/// - `light_system_program`:           Light systemprogram. verifies & applies
///   compression state transitions.
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
///     pub cpi_context_account: AccountInfo<'info>,
///     pub light_system_program: AccountInfo<'info>,
///     pub registered_program_pda: AccountInfo<'info>,
///     pub noop_program: AccountInfo<'info>,
///     pub account_compression_authority: AccountInfo<'info>,
///     pub account_compression_program: AccountInfo<'info>,
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
/// - [`ToByteArray`](light_hasher::to_byte_array::ToByteArray), which makes the struct
///   convertable to a 2D byte vector.
/// - [`DataHasher`](light_hasher::DataHasher), which makes the struct hashable
///   with the `hash()` method, based on the byte inputs from `ToByteArray`
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
/// 2. Manually implementing the `ToByteArray` trait.
///
/// # Attributes
///
/// - `skip` - skips the given field, it doesn't get included neither in
///   `AsByteVec` nor `DataHasher` implementation.
/// - `hash` - makes sure that the byte value does not exceed the BN254
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
///     #[hash]
///     c: [u8; 32],
///     #[hash]
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
#[proc_macro_derive(LightHasher, attributes(skip, hash))]
pub fn light_hasher(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    derive_light_hasher(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// SHA256 variant of the LightHasher derive macro.
///
/// This derive macro automatically implements the `DataHasher` and `ToByteArray` traits
/// for structs, using SHA256 as the hashing algorithm instead of Poseidon.
///
/// ## Example
///
/// ```rust
/// use light_sdk_macros::LightHasherSha;
/// use borsh::{BorshSerialize, BorshDeserialize};
/// use solana_pubkey::Pubkey;
///
/// #[derive(LightHasherSha, BorshSerialize, BorshDeserialize)]
/// pub struct GameState {
///     pub player: Pubkey,  // Will be hashed to 31 bytes
///     pub level: u32,
/// }
/// ```
#[proc_macro_derive(LightHasherSha, attributes(hash, skip))]
pub fn light_hasher_sha(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);

    derive_light_hasher_sha(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Alias of `LightHasher`.
#[proc_macro_derive(DataHasher, attributes(skip, hash))]
pub fn data_hasher(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    derive_light_hasher(input)
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

/// Automatically implements the HasCompressionInfo trait for structs that have a
/// `compression_info: Option<CompressionInfo>` field.
///
/// This derive macro generates the required trait methods for managing compression
/// information in compressible account structs.
///
/// ## Example
///
/// ```ignore
/// use light_sdk::compressible::{CompressionInfo, HasCompressionInfo};
///
/// #[derive(HasCompressionInfo)]
/// pub struct UserRecord {
///     pub compression_info: Option<CompressionInfo>,
///     pub owner: Pubkey,
///     pub name: String,
///     pub score: u64,
/// }
/// ```
///
/// ## Requirements
///
/// The struct must have exactly one field named `compression_info` of type
/// `Option<CompressionInfo>`.
#[proc_macro_derive(HasCompressionInfo)]
pub fn has_compression_info(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    compressible::derive_has_compression_info(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Implements CompressAs trait for custom compression behavior.
///
/// This derive macro allows you to specify which fields should be reset/overridden
/// during compression while keeping other fields as-is. Only the specified fields
/// are modified; all others retain their current values.
///
/// ## Example
///
/// ```ignore
/// use light_sdk::compressible::{CompressAs, CompressionInfo};
///
/// #[derive(CompressAs)]
/// #[compress_as(
///     start_time = 0,
///     end_time = None,
///     score = 0
/// )]
/// pub struct GameSession {
///     pub compression_info: Option<CompressionInfo>,
///     pub session_id: u64,
///     pub player: Pubkey,
///     pub game_type: String,
///     pub start_time: u64,
///     pub end_time: Option<u64>,
///     pub score: u64,
/// }
/// ```
///
/// ## Note
///
/// Use the `Compressible` derive for complete functionality - it includes this plus more.
#[proc_macro_derive(CompressAs, attributes(compress_as))]
pub fn compress_as_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    compress_as::derive_compress_as(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Automatically implements all required traits for compressible accounts.
///
/// This derive macro generates HasCompressionInfo, Size, and CompressAs trait implementations.
/// It supports optional compress_as attribute for custom compression behavior.
///
/// ## Example - Basic Usage
///
/// ```ignore
/// use light_sdk::compressible::CompressionInfo;
///
/// #[derive(Compressible)]
/// pub struct UserRecord {
///     pub compression_info: Option<CompressionInfo>,
///     pub owner: Pubkey,
///     pub name: String,
///     pub score: u64,
/// }
/// ```
///
/// ## Example - Custom Compression
///
/// ```ignore
/// #[derive(Compressible)]
/// #[compress_as(start_time = 0, end_time = None, score = 0)]
/// pub struct GameSession {
///     pub compression_info: Option<CompressionInfo>,
///     pub session_id: u64,        // KEPT
///     pub player: Pubkey,         // KEPT  
///     pub game_type: String,      // KEPT
///     pub start_time: u64,        // RESET to 0
///     pub end_time: Option<u64>,  // RESET to None
///     pub score: u64,             // RESET to 0
/// }
/// ```
#[proc_macro_derive(Compressible, attributes(compress_as))]
pub fn compressible_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    compressible_derive::derive_compressible(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
