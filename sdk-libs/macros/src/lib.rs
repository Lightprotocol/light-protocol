extern crate proc_macro;
use discriminator::discriminator;
use hasher::{derive_light_hasher, derive_light_hasher_sha};
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, ItemStruct};
use utils::into_token_stream;

mod account;
mod discriminator;
mod hasher;
mod light_pdas;
mod rent_sponsor;
mod utils;

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
    into_token_stream(light_pdas::account::traits::derive_has_compression_info(
        input,
    ))
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
    into_token_stream(light_pdas::account::traits::derive_compress_as(input))
}

/// Auto-discovering Light program macro that reads external module files.
///
/// This macro automatically discovers #[light_account(init)] fields in Accounts structs
/// by reading external module files. No explicit type list needed!
///
/// It also **automatically wraps** instruction handlers that use Light Accounts
/// structs with `light_pre_init`/`light_finalize` logic - no separate attribute needed!
///
/// Usage:
/// ```ignore
/// #[light_program]
/// #[program]
/// pub mod my_program {
///     pub mod instruction_accounts;  // Macro reads this file!
///     pub mod state;
///
///     use instruction_accounts::*;
///     use state::*;
///
///     pub fn create_user(ctx: Context<CreateUser>, params: Params) -> Result<()> {
///         // Your business logic
///     }
/// }
/// ```
///
/// The macro:
/// 1. Scans the crate's `src/` directory for `#[derive(Accounts)]` structs
/// 2. Extracts seeds from `#[account(seeds = [...])]` on `#[light_account(init)]` fields
/// 3. Auto-wraps instruction handlers that use those Accounts structs
/// 4. Generates all necessary types, enums, and instruction handlers
///
/// Seeds are declared ONCE in Anchor attributes - no duplication!
#[proc_macro_attribute]
pub fn light_program(args: TokenStream, input: TokenStream) -> TokenStream {
    let module = syn::parse_macro_input!(input as syn::ItemMod);
    into_token_stream(light_pdas::program::light_program_impl(args.into(), module))
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
    into_token_stream(light_pdas::account::traits::derive_compressible(input))
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
    into_token_stream(light_pdas::account::pack_unpack::derive_compressible_pack(
        input,
    ))
}

/// Consolidates all required traits for Light Protocol state accounts into a single derive.
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
/// use light_sdk_macros::LightAccount;
/// use light_sdk::compressible::CompressionInfo;
/// use solana_pubkey::Pubkey;
///
/// #[derive(Default, Debug, InitSpace, LightAccount)]
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
#[proc_macro_derive(LightAccount, attributes(compress_as))]
pub fn light_account_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    into_token_stream(light_pdas::account::light_compressible::derive_light_account(input))
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

/// Generates `LightFinalize` trait implementation for Light Protocol accounts.
///
/// This derive macro works alongside Anchor's `#[derive(Accounts)]` to add
/// compression finalize logic for:
/// - Accounts marked with `#[light_account(init)]` (PDAs)
/// - Accounts marked with `#[light_account(init, mint, ...)]` (compressed mints)
/// - Accounts marked with `#[rentfree_token(...)]` (rent-free token accounts)
///
/// The trait is defined in `light_sdk::compressible::LightFinalize`.
///
/// ## Usage - PDAs
///
/// ```ignore
/// #[derive(Accounts, LightAccounts)]
/// #[instruction(params: CompressionParams)]
/// pub struct CreatePda<'info> {
///     #[account(mut)]
///     pub fee_payer: Signer<'info>,
///
///     #[account(
///         init, payer = fee_payer, space = 8 + MyData::INIT_SPACE,
///         seeds = [b"my_data", authority.key().as_ref()],
///         bump
///     )]
///     #[light_account(init)]
///     pub my_account: Account<'info, MyData>,
///
///     /// CHECK: Compression config
///     pub compression_config: AccountInfo<'info>,
/// }
/// ```
///
/// ## Usage - Rent-free Token Accounts
///
/// ```ignore
/// #[derive(Accounts, LightAccounts)]
/// pub struct CreateVault<'info> {
///     #[account(
///         mut,
///         seeds = [b"vault", cmint.key().as_ref()],
///         bump
///     )]
///     #[rentfree_token(authority = [b"vault_authority"])]
///     pub vault: UncheckedAccount<'info>,
/// }
/// ```
///
/// ## Usage - Light Mints
///
/// ```ignore
/// #[derive(Accounts, LightAccounts)]
/// #[instruction(params: MintParams)]
/// pub struct CreateMint<'info> {
///     #[account(mut)]
///     pub fee_payer: Signer<'info>,
///
///     #[account(mut)]
///     #[light_account(init, mint,
///         mint_signer = mint_signer,
///         authority = authority,
///         decimals = 9,
///         mint_seeds = &[...]
///     )]
///     pub mint: UncheckedAccount<'info>,
///
///     pub mint_signer: Signer<'info>,
///     pub authority: Signer<'info>,
/// }
/// ```
///
/// ## Requirements
///
/// Your program must define:
/// - `LIGHT_CPI_SIGNER`: CPI signer pubkey constant
/// - `ID`: Program ID (from declare_id!)
///
/// The struct should have fields named `fee_payer` (or `payer`) and `compression_config`.
#[proc_macro_derive(LightAccounts, attributes(light_account, rentfree_token, instruction))]
pub fn light_accounts_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    into_token_stream(light_pdas::accounts::derive_light_accounts(input))
}
