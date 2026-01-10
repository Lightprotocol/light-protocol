extern crate proc_macro;
use accounts::{process_light_accounts, process_light_system_accounts};
use discriminator::discriminator;
use hasher::{derive_light_hasher, derive_light_hasher_sha};
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, ItemFn, ItemStruct};
use traits::process_light_traits;
use utils::into_token_stream;

mod account;
mod accounts;
mod compressible;
mod discriminator;
mod finalize;
mod hasher;
mod program;
mod rent_sponsor;
mod traits;
mod utils;

/// Adds required fields to your anchor instruction for applying a zk-compressed
/// state transition.
///
/// ## DEPRECATED
/// This macro is deprecated. Use the newer compressible instructions
/// approach with `#[add_compressible_instructions]` instead.
///
/// ## Usage
/// Add `#[light_system_accounts]` to your struct. Ensure it's applied before Anchor's
/// `#[derive(Accounts)]` and Light's `#[derive(LightTraits)]`.
///
/// ## Example
/// Note: You will have to build your program IDL using Anchor's `idl-build`
/// feature, otherwise your IDL won't include these accounts.
/// ```ignore
/// use anchor_lang::prelude::*;
/// use light_sdk_macros::light_system_accounts;
///
/// declare_id!("Fg6PaFpoGXkYsidMpWxTWKGNpKK39H3UKo7wjRZnq89u");
///
/// #[program]
/// pub mod my_program {
///     use super::*;
/// }
///
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
#[deprecated(note = "Use #[add_compressible_instructions] instead")]
#[proc_macro_attribute]
pub fn light_system_accounts(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    into_token_stream(process_light_system_accounts(input))
}

/// DEPRECATED: Use `#[add_compressible_instructions]` instead.
#[deprecated(note = "Use #[add_compressible_instructions] instead")]
#[proc_macro_attribute]
pub fn light_accounts(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    into_token_stream(process_light_accounts(input))
}

/// DEPRECATED: Use `#[add_compressible_instructions]` instead.
#[deprecated(note = "Use #[add_compressible_instructions] instead")]
#[proc_macro_derive(LightAccounts, attributes(light_account))]
pub fn light_accounts_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    into_token_stream(accounts::process_light_accounts_derive(input))
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
/// use anchor_lang::prelude::*;
/// use light_sdk_macros::LightTraits;
///
/// declare_id!("Fg6PaFpoGXkYsidMpWxTWKGNpKK39H3UKo7wjRZnq89u");
///
/// #[program]
/// pub mod my_program {
///     use super::*;
/// }
///
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
/// DEPRECATED: Use `#[add_compressible_instructions]` instead.
#[deprecated(note = "Use #[add_compressible_instructions] instead")]
#[proc_macro_derive(
    LightTraits,
    attributes(self_program, fee_payer, authority, cpi_context)
)]
pub fn light_traits_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    into_token_stream(process_light_traits(input))
}

#[proc_macro_derive(LightDiscriminator)]
pub fn light_discriminator(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    into_token_stream(discriminator(input))
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
/// will be a compilation error.
///
/// ## Example
///
/// ```ignore
/// use light_sdk_macros::LightHasher;
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
/// use light_sdk_macros::LightHasher;
/// use solana_pubkey::Pubkey;
///
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
    into_token_stream(derive_light_hasher(input))
}

/// SHA256 variant of the LightHasher derive macro.
///
/// This derive macro automatically implements the `DataHasher` and `ToByteArray` traits
/// for structs, using SHA256 as the hashing algorithm instead of Poseidon.
///
/// ## Example
///
/// ```ignore
/// use light_sdk_macros::LightHasherSha;
/// use solana_pubkey::Pubkey;
///
/// #[derive(LightHasherSha)]
/// pub struct GameState {
///     #[hash]
///     pub player: Pubkey,  // Will be hashed to 31 bytes
///     pub level: u32,
/// }
/// ```
#[proc_macro_derive(LightHasherSha, attributes(hash, skip))]
pub fn light_hasher_sha(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    into_token_stream(derive_light_hasher_sha(input))
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
/// use light_sdk_macros::HasCompressionInfo;
/// use light_compressible::CompressionInfo;
/// use solana_pubkey::Pubkey;
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
    into_token_stream(compressible::traits::derive_has_compression_info(input))
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
/// use light_sdk_macros::CompressAs;
/// use light_compressible::CompressionInfo;
/// use solana_pubkey::Pubkey;
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
    into_token_stream(compressible::traits::derive_compress_as(input))
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
/// ```ignore
/// use anchor_lang::prelude::*;
/// use light_sdk_macros::add_compressible_instructions;
///
/// declare_id!("Fg6PaFpoGXkYsidMpWxTWKGNpKK39H3UKo7wjRZnq89u");
///
/// #[add_compressible_instructions(
///     UserRecord = ("user_record", data.owner),
///     GameSession = ("game_session", data.session_id.to_le_bytes()),
///     CTokenSigner = (is_token, "ctoken_signer", ctx.fee_payer, ctx.mint)
/// )]
/// #[program]
/// pub mod my_program {
///     use super::*;
///     // Your regular instructions here - everything else is auto-generated!
///     // CTokenAccountVariant enum is automatically generated with:
///     // - CTokenSigner = 0
/// }
/// ```
#[proc_macro_attribute]
pub fn add_compressible_instructions(args: TokenStream, input: TokenStream) -> TokenStream {
    let module = syn::parse_macro_input!(input as syn::ItemMod);
    into_token_stream(compressible::instructions::add_compressible_instructions(
        args.into(),
        module,
    ))
}

/// Program-level registration of compressible accounts with auto-generated instructions.
///
/// This is an alias for `#[add_compressible_instructions]` with a cleaner name.
/// Use this on your program module to register compressible account types and their seeds.
///
/// ## Usage
///
/// ```ignore
/// use anchor_lang::prelude::*;
/// use light_sdk_macros::compressible;
///
/// #[compressible(
///     // PDAs: AccountType = (SEED_CONST, ctx.accounts.field, data.param, ...)
///     PoolState = (POOL_SEED, ctx.accounts.amm_config, ctx.accounts.token_0_mint, ctx.accounts.token_1_mint),
///     
///     // Tokens: is_token flag + seeds + authority (required for CPI signing)
///     Token0Vault = (is_token, POOL_VAULT_SEED, ctx.accounts.pool_state, ctx.accounts.token_0_mint, authority = AUTH_SEED),
///     
///     // Instruction data fields (when using data.*)
///     owner = Pubkey,
///     session_id = u64,
/// )]
/// #[program]
/// pub mod my_program {
///     use super::*;
///     // Your instructions here - compress/decompress instructions are auto-generated
/// }
/// ```
///
/// ## What Gets Generated
///
/// - `DecompressAccountsIdempotent` - Accounts struct with optional seed accounts
/// - `CompressAccountsIdempotent` - Accounts struct for compression
/// - `SeedParams` - Struct for instruction data used in seeds
/// - `CompressedAccountVariant` - Enum of all compressible account types
/// - `CTokenAccountVariant` - Enum of token account types
/// - `decompress_accounts_idempotent` - Instruction handler
/// - `compress_accounts_idempotent` - Instruction handler
/// - `initialize_compression_config` - Config initialization
/// - `update_compression_config` - Config update
/// - Client helper functions for PDA derivation
#[proc_macro_attribute]
pub fn compressible(args: TokenStream, input: TokenStream) -> TokenStream {
    let module = syn::parse_macro_input!(input as syn::ItemMod);
    into_token_stream(compressible::instructions::add_compressible_instructions(
        args.into(),
        module,
    ))
}

#[proc_macro_attribute]
pub fn account(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    into_token_stream(account::account(input))
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
/// use light_compressible::CompressionInfo;
/// use solana_pubkey::Pubkey;
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
/// use light_sdk_macros::Compressible;
/// use light_compressible::CompressionInfo;
/// use solana_pubkey::Pubkey;
///
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
#[proc_macro_derive(Compressible, attributes(compress_as, light_seeds))]
pub fn compressible_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    into_token_stream(compressible::traits::derive_compressible(input))
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
/// use light_compressible::CompressionInfo;
/// use solana_pubkey::Pubkey;
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
    into_token_stream(compressible::pack_unpack::derive_compressible_pack(input))
}

/// Consolidates all required traits for compressible accounts into a single derive.
///
/// This macro is equivalent to deriving:
/// - `LightHasherSha` (SHA256/ShaFlat hashing - type 3)
/// - `LightDiscriminator` (unique discriminator)
/// - `Compressible` (HasCompressionInfo + CompressAs + Size + CompressedInitSpace)
/// - `CompressiblePack` (Pack + Unpack + Packed struct generation)
///
/// ## Example
///
/// ```ignore
/// use light_sdk_macros::LightCompressible;
/// use light_sdk::compressible::CompressionInfo;
/// use solana_pubkey::Pubkey;
///
/// #[derive(Default, Debug, InitSpace, LightCompressible)]
/// #[account]
/// pub struct UserRecord {
///     pub owner: Pubkey,
///     #[max_len(32)]
///     pub name: String,
///     pub score: u64,
///     pub compression_info: Option<CompressionInfo>,
/// }
/// ```
///
/// This is equivalent to:
/// ```ignore
/// #[derive(Default, Debug, InitSpace, LightHasherSha, LightDiscriminator, Compressible, CompressiblePack)]
/// #[account]
/// pub struct UserRecord { ... }
/// ```
///
/// ## Attributes
///
/// - `#[compress_as(...)]` - Optional: specify field values to reset during compression
///
/// ## Notes
///
/// - The `compression_info` field is auto-detected and handled (no `#[skip]` needed)
/// - SHA256 (ShaFlat) hashes the entire serialized struct (no `#[hash]` needed)
/// - The struct must have a `compression_info: Option<CompressionInfo>` field
#[proc_macro_derive(LightCompressible, attributes(compress_as))]
pub fn light_compressible(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    into_token_stream(compressible::light_compressible::derive_light_compressible(input))
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
/// use light_sdk_macros::{Compressible, CompressiblePack};
/// use solana_pubkey::Pubkey;
///
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
/// # macro_rules! compressed_account_variant_with_instructions { ($($t:ty),*) => {} }
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
///
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
/// use anchor_lang::prelude::*;
/// use light_sdk_macros::DecompressContext;
///
/// declare_id!("Fg6PaFpoGXkYsidMpWxTWKGNpKK39H3UKo7wjRZnq89u");
///
/// struct UserRecord;
/// struct GameSession;
/// enum CTokenAccountVariant {}
///
/// #[derive(Accounts, DecompressContext)]
/// #[pda_types(UserRecord, GameSession)]
/// #[token_variant(CTokenAccountVariant)]
/// pub struct DecompressAccountsIdempotent<'info> {
///     #[account(mut)]
///     pub fee_payer: Signer<'info>,
///     pub config: AccountInfo<'info>,
///     #[account(mut)]
///     pub rent_sponsor: Signer<'info>,
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
    into_token_stream(compressible::decompress_context::derive_decompress_context(
        input,
    ))
}

/// Derives a Rent Sponsor PDA for a program at compile time.
///
/// Seeds: ["rent_sponsor", <u16 version little-endian>]
///
/// ## Example
///
/// ```ignore
/// use light_sdk_macros::derive_light_rent_sponsor_pda;
///
/// pub const RENT_SPONSOR_DATA: ([u8; 32], u8) =
///     derive_light_rent_sponsor_pda!("8Ld9pGkCNfU6A7KdKe1YrTNYJWKMCFqVHqmUvjNmER7B", 1);
/// ```
#[proc_macro]
pub fn derive_light_rent_sponsor_pda(input: TokenStream) -> TokenStream {
    rent_sponsor::derive_light_rent_sponsor_pda(input)
}

/// Derives a complete Rent Sponsor configuration for a program at compile time.
///
/// Returns ::light_sdk_types::RentSponsor { program_id, rent_sponsor, bump, version }.
///
/// ## Example
///
/// ```ignore
/// use light_sdk_macros::derive_light_rent_sponsor;
///
/// pub const RENT_SPONSOR: ::light_sdk_types::RentSponsor =
///     derive_light_rent_sponsor!("8Ld9pGkCNfU6A7KdKe1YrTNYJWKMCFqVHqmUvjNmER7B", 1);
/// ```
#[proc_macro]
pub fn derive_light_rent_sponsor(input: TokenStream) -> TokenStream {
    rent_sponsor::derive_light_rent_sponsor(input)
}
/// DEPRECATED: This macro is not used. Use standard Anchor `#[program]` with
/// `#[add_compressible_instructions]` instead.
///
/// ## Example
///
/// ```ignore
/// use light_sdk_macros::light_program;
/// use anchor_lang::prelude::*;
///
/// declare_id!("Fg6PaFpoGXkYsidMpWxTWKGNpKK39H3UKo7wjRZnq89u");
///
/// #[derive(Accounts)]
/// pub struct MyInstruction {}
///
/// #[light_program]
/// pub mod my_program {
///     use super::*;
///     pub fn my_instruction(ctx: Context<MyInstruction>) -> Result<()> {
///         // Your instruction logic here
///         Ok(())
///     }
/// }
/// ```
#[deprecated(note = "Use standard Anchor #[program] with #[add_compressible_instructions] instead")]
#[proc_macro_attribute]
pub fn light_program(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ItemMod);
    into_token_stream(program::program(input))
}

/// Generates `LightFinalize` trait implementation for compressible accounts and light-mints.
///
/// This derive macro works alongside Anchor's `#[derive(Accounts)]` to add
/// compression finalize logic for:
/// - Accounts marked with `#[compressible(...)]` (compressible PDAs)
/// - Accounts marked with `#[light_mint(...)]` (light-mint creation)
///
/// The trait is defined in `light_sdk::compressible::LightFinalize`.
///
/// ## Usage - Compressible PDAs
///
/// ```ignore
/// #[derive(Accounts, LightFinalize)]
/// #[instruction(params: CompressionParams)]
/// pub struct CreateCompressible<'info> {
///     #[account(mut)]
///     pub fee_payer: Signer<'info>,
///     
///     #[account(init, payer = fee_payer, space = 8 + MyData::INIT_SPACE)]
///     #[compressible(
///         address_tree_info = params.address_tree_info,
///         output_tree = 0
///     )]
///     pub my_account: Account<'info, MyData>,
///     
///     /// CHECK: Compression config
///     pub compression_config: AccountInfo<'info>,
/// }
/// ```
///
/// ## Usage - Light Mints
///
/// ```ignore
/// #[derive(Accounts, LightFinalize)]
/// #[instruction(params: MintParams)]
/// pub struct CreateMint<'info> {
///     #[account(mut)]
///     pub fee_payer: Signer<'info>,
///     
///     #[account(init, /* ... */)]
///     #[light_mint(
///         mint_signer = mint_signer,
///         authority = authority,
///         decimals = 9,
///         address_tree_info = params.address_tree_info,
///         output_tree = 0
///     )]
///     pub mint: InterfaceAccount<'info, Mint>,
///     
///     pub mint_signer: Signer<'info>,
///     pub authority: Signer<'info>,
/// }
/// ```
///
/// ## Usage - Mixed (PDAs + Mints)
///
/// Multiple compressible PDAs and light-mints can be created in the same instruction.
/// They are batched together using CPI context for a single proof execution.
///
/// ```ignore
/// #[derive(Accounts, LightFinalize)]
/// #[instruction(params: InitParams)]
/// pub struct Initialize<'info> {
///     #[account(mut)]
///     pub fee_payer: Signer<'info>,
///     
///     #[account(init, payer = fee_payer, space = 8 + Config::INIT_SPACE)]
///     #[compressible(address_tree_info = params.address_tree_info, output_tree = 0)]
///     pub config: Account<'info, Config>,
///     
///     #[account(init, /* ... */)]
///     #[light_mint(
///         mint_signer = mint_signer,
///         authority = authority,
///         decimals = 9,
///         address_tree_info = params.address_tree_info,
///         output_tree = 1
///     )]
///     pub token_mint: InterfaceAccount<'info, Mint>,
///     
///     // ... other accounts
/// }
///
/// #[light_instruction(params)]
/// pub fn initialize(ctx: Context<Initialize>, params: InitParams) -> Result<()> {
///     // Your logic
///     Ok(())
///     // light_finalize auto-called: batches PDA + mint creation with single proof
/// }
/// ```
///
/// ## Attributes
///
/// ### `#[compressible(...)]` for PDAs:
/// - `address_tree_info` (required): Expression for address tree info
/// - `output_tree` (required): Output state tree index
///
/// ### `#[light_mint(...)]` for mints:
/// - `mint_signer` (required): The signer account used as mint seed
/// - `authority` (required): Mint authority
/// - `decimals` (required): Token decimals
/// - `address_tree_info` (required): Address tree info expression
/// - `output_tree` (required): Output state tree index
/// - `freeze_authority` (optional): Freeze authority
///
/// ## Requirements
///
/// Your program must define:
/// - `LIGHT_CPI_SIGNER`: CPI signer pubkey constant
/// - `ID`: Program ID (from declare_id!)
///
/// The struct should have fields named `fee_payer` (or `payer`) and `compression_config`.
#[proc_macro_derive(LightFinalize, attributes(compressible, light_mint, instruction))]
pub fn light_finalize_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    into_token_stream(finalize::derive_light_finalize(input))
}

/// Attribute macro that auto-calls `light_finalize()` at end of instruction handler.
///
/// This macro wraps your instruction handler to automatically call the
/// `LightFinalize::light_finalize()` method before returning, which executes
/// the compression CPIs. This runs BEFORE Anchor's `exit()` hook.
///
/// ## Usage
///
/// ```ignore
/// use anchor_lang::prelude::*;
/// use light_sdk_macros::light_instruction;
///
/// // The argument is the name of the parameter containing compression data
/// #[light_instruction(params)]
/// pub fn create_compressible(ctx: Context<CreateCompressible>, params: CompressionParams) -> Result<()> {
///     // Your business logic
///     ctx.accounts.my_account.value = params.value;
///     
///     // light_finalize() is auto-called here before returning
///     Ok(())
/// }
/// ```
///
/// ## How It Works
///
/// The macro transforms your function to:
/// 1. Execute your original function body
/// 2. On success, call `ctx.accounts.light_finalize(ctx.remaining_accounts, &params)`
/// 3. Return the result
///
/// This ensures compression CPIs run after your logic but before Anchor serializes accounts.
///
/// ## Important Notes
///
/// - The `params` argument must match a parameter name in your function signature
/// - Your accounts struct must derive `LightFinalize`
/// - Use `?` operator for error handling (not explicit `return Err(...)`)
/// - Errors will skip `light_finalize` and propagate normally
#[proc_macro_attribute]
pub fn light_instruction(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as finalize::instruction::LightInstructionArgs);
    let item = parse_macro_input!(input as ItemFn);
    into_token_stream(finalize::instruction::light_instruction_impl(args, item))
}
