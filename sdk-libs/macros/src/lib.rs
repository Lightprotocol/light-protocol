extern crate proc_macro;
use accounts::{process_light_accounts, process_light_system_accounts};
use discriminator::discriminator;
use hasher::{derive_light_hasher, derive_light_hasher_sha};
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, ItemStruct};
use traits::process_light_traits;

mod account;
mod account_seeds;
mod accounts;
mod client_seed_functions;
mod compress_as;
mod compressible;
mod compressible_derive;
mod compressible_instructions;
mod compressible_instructions_compress;
mod compressible_instructions_decompress;
mod cpi_signer;
mod ctoken_seed_generation;
mod derive_decompress_context;
// Legacy CToken and instruction generator modules removed - functionality integrated into compressible_instructions
mod derive_seeds;
mod discriminator;
mod hasher;

mod pack_unpack;
mod program;
mod traits;
mod variant_enum;

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

// /// SHA256 variant of the LightDiscriminator derive macro.
// ///
// /// This derive macro provides the same discriminator functionality as LightDiscriminator
// /// but is designed to be used with SHA256-based hashing for consistency.
// ///
// /// ## Example
// ///
// /// ```ignore
// /// use light_sdk::sha::{LightHasher, LightDiscriminator};
// ///
// /// #[derive(LightHasher, LightDiscriminator)]
// /// pub struct LargeGameState {
// ///     pub field1: u64, pub field2: u64, pub field3: u64, pub field4: u64,
// ///     pub field5: u64, pub field6: u64, pub field7: u64, pub field8: u64,
// ///     pub field9: u64, pub field10: u64, pub field11: u64, pub field12: u64,
// ///     pub field13: u64, pub field14: u64, pub field15: u64,
// ///     pub owner: Pubkey,
// ///     pub authority: Pubkey,
// /// }
// /// ```
// #[proc_macro_derive(LightDiscriminatorSha)]
// pub fn light_discriminator_sha(input: TokenStream) -> TokenStream {
//     let input = parse_macro_input!(input as ItemStruct);
//     discriminator_sha(input)
//         .unwrap_or_else(|err| err.to_compile_error())
//         .into()
// }

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

/// Legacy CompressAs trait implementation (use Compressible instead).
///
/// This derive macro allows you to specify which fields should be reset/overridden
/// during compression while keeping other fields as-is. Only the specified fields
/// are modified; all others retain their current values.
///
/// ## Example
///
/// ```ignore
/// use light_sdk::compressible::{CompressAs, CompressionInfo, HasCompressionInfo};
/// use light_sdk_macros::CompressAs;
///
/// #[derive(CompressAs)]
/// #[compress_as(
///     start_time = 0,
///     end_time = None,
///     score = 0
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
/// ## Note
///
/// Use the new `Compressible` derive instead - it includes this functionality plus more.
#[proc_macro_derive(CompressAs, attributes(compress_as))]
pub fn compress_as_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);

    compress_as::derive_compress_as(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Adds compressible account support with automatic seed generation.
///
/// This macro generates everything needed for compressible accounts:
/// - CompressedAccountVariant enum with all trait implementations  
/// - Compress and decompress instructions with auto-generated seed derivation
/// - CTokenSeedProvider implementation for token accounts
/// - All required account structs and functions
///
/// ## Usage
/// ```
/// #[add_compressible_instructions(
///     UserRecord = ("user_record", data.owner),
///     GameSession = ("game_session", data.session_id.to_le_bytes()),
///     CTokenSigner = (is_token, "ctoken_signer", ctx.fee_payer, ctx.mint)
/// )]
/// #[program]
/// pub mod my_program {
///     // Your regular instructions here - everything else is auto-generated!
///     // CTokenAccountVariant enum is automatically generated with:
///     // - CTokenSigner = 0
/// }
/// ```
#[proc_macro_attribute]
pub fn add_compressible_instructions(args: TokenStream, input: TokenStream) -> TokenStream {
    let module = syn::parse_macro_input!(input as syn::ItemMod);
    compressible_instructions::add_compressible_instructions(args.into(), module)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

// /// Adds native compressible instructions for the specified account types
// ///
// /// This macro generates thin wrapper processor functions that you dispatch manually.
// ///
// /// ## Usage
// /// ```
// /// #[add_native_compressible_instructions(MyPdaAccount, AnotherAccount)]
// /// pub mod compression {}
// /// ```
// ///
// /// This generates:
// /// - Unified data structures (CompressedAccountVariant enum, etc.)
// /// - Instruction data structs (CreateCompressionConfigData, etc.)
// /// - Processor functions (create_compression_config, compress_my_pda_account, etc.)
// ///
// /// You then dispatch these in your process_instruction function.
// #[proc_macro_attribute]
// pub fn add_native_compressible_instructions(args: TokenStream, input: TokenStream) -> TokenStream {
//     let input = syn::parse_macro_input!(input as syn::ItemMod);

//     native_compressible::add_native_compressible_instructions(args.into(), input)
//         .unwrap_or_else(|err| err.to_compile_error())
//         .into()
// }
#[proc_macro_attribute]
pub fn account(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);

    account::account(input)
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
/// use light_sdk_macros::Compressible;
/// use light_sdk::compressible::CompressionInfo;
///
/// #[derive(Compressible)]
/// pub struct UserRecord {
///     #[skip]
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
///     #[skip]
///     pub compression_info: Option<CompressionInfo>,
///     pub session_id: u64,        // KEPT
///     pub player: Pubkey,         // KEPT  
///     pub game_type: String,      // KEPT
///     pub start_time: u64,        // RESET to 0
///     pub end_time: Option<u64>,  // RESET to None
///     pub score: u64,             // RESET to 0
/// }
/// ```
#[proc_macro_derive(Compressible, attributes(compress_as, light_seeds))]
pub fn compressible_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    compressible_derive::derive_compressible(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Automatically implements Pack and Unpack traits for compressible accounts.
///
/// For types with Pubkey fields, generates a PackedXxx struct and proper packing.
/// For types without Pubkeys, generates identity Pack/Unpack implementations.
///
/// ## Example
///
/// ```ignore
/// use light_sdk_macros::CompressiblePack;
///
/// #[derive(CompressiblePack)]
/// pub struct UserRecord {
///     pub compression_info: Option<CompressionInfo>,
///     pub owner: Pubkey,  // Will be packed as u8 index
///     pub name: String,   // Kept as-is
///     pub score: u64,     // Kept as-is
/// }
/// // This generates PackedUserRecord struct + Pack/Unpack implementations
/// ```
#[proc_macro_derive(CompressiblePack)]
pub fn compressible_pack(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    pack_unpack::derive_compressible_pack(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

// DEPRECATED: compressed_account_variant macro is now integrated into add_compressible_instructions
// Use add_compressible_instructions instead for complete automation

/// Generates complete compressible instructions with auto-generated seed derivation.
///
/// This is a drop-in replacement for manual decompress_accounts_idempotent and
/// compress_accounts_idempotent instructions. It reads #[light_seeds(...)] attributes
/// from account types and generates complete instructions with inline seed derivation.
///
/// ## Example
///
/// Add #[light_seeds(...)] to your account types:
/// ```ignore
/// #[derive(Compressible, CompressiblePack)]
/// #[light_seeds(b"user_record", owner.as_ref())]
/// pub struct UserRecord {
///     pub owner: Pubkey,
///     // ...
/// }
///
/// #[derive(Compressible, CompressiblePack)]  
/// #[light_seeds(b"game_session", session_id.to_le_bytes().as_ref())]
/// pub struct GameSession {
///     pub session_id: u64,
///     // ...
/// }
/// ```
///
/// Then generate complete instructions:
/// ```ignore
/// compressed_account_variant_with_instructions!(UserRecord, GameSession, PlaceholderRecord);
/// ```
///
/// This generates:
/// - CompressedAccountVariant enum + all trait implementations
/// - Complete decompress_accounts_idempotent instruction with auto-generated seed derivation
/// - Complete compress_accounts_idempotent instruction with auto-generated seed derivation
/// - CompressedAccountData struct
///
/// The generated instructions automatically handle seed derivation for each account type
/// without requiring manual seed function calls.
// DEPRECATED: compressed_account_variant_with_instructions macro is now integrated into add_compressible_instructions
// Use add_compressible_instructions instead for complete automation with declarative seed syntax
/// Generates seed getter functions by analyzing Anchor account structs.
///
/// This macro scans account structs for `#[account(seeds = [...], ...)]` attributes
/// and generates corresponding public seed getter functions that can be used by
/// both the program and external clients.
///
/// ## Example
///
/// ```ignore
/// use light_sdk_macros::generate_seed_functions;
///
/// generate_seed_functions! {
///     #[derive(Accounts)]
///     pub struct CreateRecord<'info> {
///         #[account(
///             init,
///             seeds = [b"user_record", user.key().as_ref()],
///             bump,
///         )]
///         pub user_record: Account<'info, UserRecord>,
///         pub user: Signer<'info>,
///     }
///
///     #[derive(Accounts)]
///     #[instruction(session_id: u64)]
///     pub struct CreateGameSession<'info> {
///         #[account(
///             init,
///             seeds = [b"game_session", session_id.to_le_bytes().as_ref()],
///             bump,
///         )]
///         pub game_session: Account<'info, GameSession>,
///         pub player: Signer<'info>,
///     }
/// }
/// ```
///
/// This generates:
/// - `get_user_record_seeds(user: &Pubkey) -> (Vec<Vec<u8>>, Pubkey)`
/// - `get_game_session_seeds(session_id: u64) -> (Vec<Vec<u8>>, Pubkey)`
///
/// The functions extract parameters from the seeds expressions and create
/// public functions that match the exact same seed derivation logic.
#[proc_macro]
pub fn generate_seed_functions(input: TokenStream) -> TokenStream {
    account_seeds::generate_seed_functions(input.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

// Legacy add_compressible_instructions_enhanced macro removed - now just use add_compressible_instructions!

// DEPRECATED: ctoken_seeds macro is now integrated into add_compressible_instructions
// Use add_compressible_instructions with CToken seed specifications instead

/// Automatically generates seed getter functions for PDA and token accounts.
///
/// This derive macro generates public functions that can be used by both the program
/// and external clients to get PDA seeds and addresses.
///
/// ## Example - PDA Account
///
/// ```ignore
/// use light_sdk_macros::DeriveSeeds;
///
/// #[derive(DeriveSeeds)]
/// #[seeds("user_record", owner)]
/// pub struct UserRecord {
///     pub owner: Pubkey,
///     pub name: String,
///     pub score: u64,
/// }
/// // Generates: get_user_record_seeds(owner: &Pubkey) -> (Vec<Vec<u8>>, Pubkey)
/// ```
///
/// ## Example - Token Account
///
/// ```ignore
/// #[derive(DeriveSeeds)]
/// #[seeds("ctoken_signer", user, mint)]
/// #[token_account]
/// pub struct CTokenSigner {
///     pub user: Pubkey,
///     pub mint: Pubkey,
/// }
/// // Generates: get_c_token_signer_seeds(user: &Pubkey, mint: &Pubkey) -> (Vec<Vec<u8>>, Pubkey)
/// ```
///
/// ## Supported Seed Types
///
/// - String literals: `"user_record"` -> `b"user_record".as_ref()`
/// - Pubkey fields: `owner` -> `owner.as_ref()`
/// - u64 fields: `session_id` -> `session_id.to_le_bytes().as_ref()`
/// - Custom expressions: `custom_expr` -> `custom_expr`
#[proc_macro_derive(DeriveSeeds, attributes(seeds, token_account))]
pub fn derive_seeds(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    derive_seeds::derive_seeds(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Derive DecompressContext trait implementation.
///
/// This generates the full DecompressContext trait implementation for
/// decompression account structs. Can be used standalone or is automatically
/// used by add_compressible_instructions.
///
/// ## Attributes
/// - `#[pda_types(Type1, Type2, ...)]` - List of PDA account types
/// - `#[token_variant(CTokenAccountVariant)]` - The token variant enum name
///
/// ## Example
///
/// ```ignore
/// #[derive(Accounts, DecompressContext)]
/// #[pda_types(UserRecord, GameSession)]
/// #[token_variant(CTokenAccountVariant)]
/// pub struct DecompressAccountsIdempotent<'info> {
///     #[account(mut)]
///     pub fee_payer: Signer<'info>,
///     pub config: AccountInfo<'info>,
///     #[account(mut)]
///     pub rent_payer: Signer<'info>,
///     #[account(mut)]
///     pub ctoken_rent_sponsor: AccountInfo<'info>,
///     pub ctoken_program: UncheckedAccount<'info>,
///     pub ctoken_cpi_authority: UncheckedAccount<'info>,
///     pub ctoken_config: UncheckedAccount<'info>,
/// }
/// ```
#[proc_macro_derive(DecompressContext, attributes(pda_types, token_variant))]
pub fn derive_decompress_context(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    derive_decompress_context::derive_decompress_context(input)
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
