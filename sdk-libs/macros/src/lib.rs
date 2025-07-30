extern crate proc_macro;
use accounts::{process_light_accounts, process_light_system_accounts};
use discriminator::{discriminator, discriminator_sha};
use hasher::{derive_light_hasher, derive_light_hasher_sha};
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, ItemStruct};
use traits::process_light_traits;

mod account;
mod accounts;
mod compress_as;
mod compressible;
mod cpi_signer;
mod discriminator;
mod hasher;
mod native_compressible;
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
    discriminator(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// SHA256 variant of the LightDiscriminator derive macro.
///
/// This derive macro provides the same discriminator functionality as LightDiscriminator
/// but is designed to be used with SHA256-based hashing for consistency.
///
/// ## Example
///
/// ```ignore
/// use light_sdk::sha::{LightHasher, LightDiscriminator};
///
/// #[derive(LightHasher, LightDiscriminator)]
/// pub struct LargeGameState {
///     pub field1: u64, pub field2: u64, pub field3: u64, pub field4: u64,
///     pub field5: u64, pub field6: u64, pub field7: u64, pub field8: u64,
///     pub field9: u64, pub field10: u64, pub field11: u64, pub field12: u64,
///     pub field13: u64, pub field14: u64, pub field15: u64,
///     pub owner: Pubkey,
///     pub authority: Pubkey,
/// }
/// ```
#[proc_macro_derive(LightDiscriminatorSha)]
pub fn light_discriminator_sha(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    discriminator_sha(input)
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
/// will be a compilation error.
///
/// ## Example
///
/// ```ignore
/// use light_sdk::LightHasher;
/// use solana_pubkey::Pubkey;
///
/// #[derive(LightHasher)]
/// pub struct UserRecord {
///     pub owner: Pubkey,
///     pub name: String,
///     pub score: u64,
/// }
/// ```
///
/// ## Hash attribute
///
/// Fields marked with `#[hash]` will be hashed to field size (31 bytes) before
/// being included in the main hash calculation. This is useful for fields that
/// exceed the field size limit (like Pubkeys which are 32 bytes).
///
/// ```ignore
/// #[derive(LightHasher)]
/// pub struct GameState {
///     #[hash]
///     pub player: Pubkey,  // Will be hashed to 31 bytes
///     pub level: u32,
/// }
/// ```
#[proc_macro_derive(LightHasher, attributes(hash, skip))]
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
/// ```ignore
/// use light_sdk::sha::LightHasher;
///
/// #[derive(LightHasher)]
/// pub struct GameState {
///     #[hash]
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

    derive_light_hasher_sha(input)
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
///     #[skip]
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
/// `Option<CompressionInfo>`. The field should be marked with `#[skip]` to
/// exclude it from hashing.
#[proc_macro_derive(HasCompressionInfo)]
pub fn has_compression_info(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);

    compressible::derive_has_compression_info(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Automatically implements the CompressAs trait for structs with custom compression logic.
///
/// This derive macro allows you to specify which fields should be reset/overridden
/// during compression while keeping other fields as-is. Only the specified fields
/// are modified; all others retain their current values.
///
/// ## Example
///
/// ```ignore
/// use light_sdk::compressible::{CompressAs, CompressionInfo, HasCompressionInfo};
/// use light_sdk_macros::{CompressAs, HasCompressionInfo};
///
/// #[derive(CompressAs, HasCompressionInfo)]
/// #[compressible_as(
///     start_time = 0,
///     end_time = None,
///     score = 0
///     // All other fields (session_id, player, game_type, compression_info)
///     // are kept as-is automatically
/// )]
/// pub struct GameSession {
///     #[skip]
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
/// ## Usage with add_compressible_instructions
///
/// When a struct implements CompressAs (via this derive), the `add_compressible_instructions`
/// macro will ONLY generate the custom compression instruction (`compress_mystruct_with_custom_data`).
/// The regular compression instruction (`compress_mystruct`) will NOT be generated.
///
/// ## Requirements
///
/// - The struct must have named fields
/// - All overridden field values must be valid expressions for the field types
/// - The struct should also derive `HasCompressionInfo` for full compatibility
/// - Must include `#[compressible_as(...)]` attribute with field overrides
#[proc_macro_derive(CompressAs, attributes(compressible_as))]
pub fn compress_as(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);

    compress_as::derive_compress_as(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Adds compress instructions for the specified account types (Anchor version)
///
/// This macro must be placed BEFORE the #[program] attribute to ensure
/// the generated instructions are visible to Anchor's macro processing.
///
/// ## Usage
/// ```
/// #[add_compressible_instructions(UserRecord, GameSession)]
/// #[program]
/// pub mod my_program {
///     // Your regular instructions here
/// }
/// ```
#[proc_macro_attribute]
pub fn add_compressible_instructions(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::ItemMod);

    compressible::add_compressible_instructions(args.into(), input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Adds native compressible instructions for the specified account types
///
/// This macro generates thin wrapper processor functions that you dispatch manually.
///
/// ## Usage
/// ```
/// #[add_native_compressible_instructions(MyPdaAccount, AnotherAccount)]
/// pub mod compression {}
/// ```
///
/// This generates:
/// - Unified data structures (CompressedAccountVariant enum, etc.)
/// - Instruction data structs (CreateCompressionConfigData, etc.)
/// - Processor functions (create_compression_config, compress_my_pda_account, etc.)
///
/// You then dispatch these in your process_instruction function.
#[proc_macro_attribute]
pub fn add_native_compressible_instructions(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::ItemMod);

    native_compressible::add_native_compressible_instructions(args.into(), input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn account(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);

    account::account(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Derive the CPI signer from the program ID. The program ID must be a string
/// literal.
///
/// ## Example
///
/// ```ignore
/// use light_sdk::derive_light_cpi_signer;
///
/// pub const LIGHT_CPI_SIGNER: CpiSigner =
///     derive_light_cpi_signer!("8Ld9pGkCNfU6A7KdKe1YrTNYJWKMCFqVHqmUvjNmER7B");
/// ```
#[proc_macro]
pub fn derive_light_cpi_signer(input: TokenStream) -> TokenStream {
    cpi_signer::derive_light_cpi_signer(input)
}

/// Generates a Light program for the given module.
///
/// ## Example
///
/// ```ignore
/// use light_sdk::light_program;
///
/// #[light_program]
/// pub mod my_program {
///     pub fn my_instruction(ctx: Context<MyInstruction>) -> Result<()> {
///         // Your instruction logic here
///         Ok(())
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn light_program(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ItemMod);

    program::program(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
