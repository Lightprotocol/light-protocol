extern crate proc_macro;
use discriminator::{anchor_discriminator, light_discriminator};
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

#[cfg(test)]
mod light_pdas_tests;

/// Derives a discriminator using SHA256("{struct_name}")[0..8].
///
/// This is the Light Protocol native discriminator format.
/// Use this for new Light Protocol accounts that don't need Anchor compatibility.
///
/// ## Example
///
/// ```ignore
/// use light_sdk::LightDiscriminator;
///
/// #[derive(LightDiscriminator)]
/// pub struct MyAccount {
///     pub owner: Pubkey,
///     pub counter: u64,
/// }
/// // MyAccount::LIGHT_DISCRIMINATOR = SHA256("MyAccount")[0..8]
/// ```
#[proc_macro_derive(LightDiscriminator)]
pub fn light_discriminator_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    into_token_stream(light_discriminator(input))
}

/// Derives a discriminator using SHA256("account:{struct_name}")[0..8].
///
/// This is the Anchor-compatible discriminator format.
/// Use this when you need compatibility with Anchor's account discriminator format.
///
/// ## Example
///
/// ```ignore
/// use light_sdk::AnchorDiscriminator;
///
/// #[derive(AnchorDiscriminator)]
/// pub struct MyAccount {
///     pub owner: Pubkey,
///     pub counter: u64,
/// }
/// // MyAccount::LIGHT_DISCRIMINATOR = SHA256("account:MyAccount")[0..8]
/// ```
#[proc_macro_derive(AnchorDiscriminator)]
pub fn anchor_discriminator_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    into_token_stream(anchor_discriminator(input))
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

/// Derive macro for manually specifying compressed account variants on an enum.
///
/// Generates equivalent code to `#[light_program]` auto-discovery, but allows
/// specifying account types and seeds explicitly. Useful for external programs
/// where you don't own the module.
///
/// ## Example
///
/// ```ignore
/// #[derive(LightProgram)]
/// pub enum ProgramAccounts {
///     #[light_account(pda::seeds = [b"record", ctx.owner])]
///     Record(MinimalRecord),
///
///     #[light_account(pda::seeds = [RECORD_SEED, ctx.owner], pda::zero_copy)]
///     ZeroCopyRecord(ZeroCopyRecord),
///
///     #[light_account(token::seeds = [VAULT_SEED, ctx.mint], token::owner_seeds = [AUTH_SEED])]
///     Vault,
///
///     #[light_account(associated_token)]
///     Ata,
/// }
/// ```
///
/// Seed expressions use explicit prefixes:
/// - `ctx.field` - context account reference
/// - `data.field` - instruction data parameter
/// - `b"literal"` or `"literal"` - byte/string literal
/// - `CONSTANT` or `path::CONSTANT` - constant in SCREAMING_SNAKE_CASE
#[proc_macro_derive(LightProgram, attributes(light_account))]
pub fn light_program_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    into_token_stream(light_pdas::program::derive_light_program_impl(input))
}

/// Pinocchio variant of `#[derive(LightProgram)]`.
///
/// Generates pinocchio-compatible code instead of Anchor:
/// - `BorshSerialize/BorshDeserialize` instead of `AnchorSerialize/AnchorDeserialize`
/// - `light_account_pinocchio::` paths instead of `light_account::`
/// - Config/compress/decompress as enum associated functions
/// - `[u8; 32]` instead of `Pubkey` in generated params
///
/// See `#[derive(LightProgram)]` for usage syntax (identical attribute syntax).
#[proc_macro_derive(LightProgramPinocchio, attributes(light_account))]
pub fn light_program_pinocchio_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    into_token_stream(light_pdas::program::derive_light_program_pinocchio_impl(
        input,
    ))
}

#[proc_macro_attribute]
pub fn account(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    into_token_stream(account::account(input))
}

/// Generates a unified `LightAccount` trait implementation for light account structs.
///
/// This macro generates:
/// - `LightHasherSha` (SHA256/ShaFlat hashing via DataHasher + ToByteArray)
/// - `LightDiscriminator` (unique 8-byte discriminator)
/// - `impl LightAccount for T` (unified trait with pack/unpack, compression_info accessors)
/// - `PackedT` struct (Pubkeys -> u8 indices, compression_info excluded to save 24 bytes)
///
/// ## Example
///
/// ```ignore
/// use light_sdk_macros::{LightAccount, LightDiscriminator, LightHasherSha};
/// use light_account::CompressionInfo;
/// use solana_pubkey::Pubkey;
///
/// #[derive(Default, Debug, InitSpace, LightAccount, LightDiscriminator, LightHasherSha)]
/// #[account]
/// pub struct UserRecord {
///     pub compression_info: CompressionInfo,  // Non-Option, first or last field
///     pub owner: Pubkey,
///     #[max_len(32)]
///     pub name: String,
///     pub score: u64,
/// }
/// ```
///
/// ## Generated Code
///
/// The macro generates:
/// - `PackedUserRecord` struct with Pubkeys replaced by u8 indices and compression_info excluded
/// - `impl LightAccount for UserRecord` with:
///   - `const ACCOUNT_TYPE: AccountType = AccountType::Pda`
///   - `const INIT_SPACE: usize` (from Anchor's Space trait)
///   - `fn compression_info(&self)` / `fn compression_info_mut(&mut self)`
///   - `fn set_decompressed(&mut self, config, slot)` (resets transient fields)
///   - `fn pack(&self, accounts)` / `fn unpack(packed, accounts)`
/// - Compile-time assertion that INIT_SPACE <= 800 bytes
///
/// ## Attributes
///
/// - `#[compress_as(field = value)]` - Optional: reset field values during set_decompressed
/// - `#[skip]` - Exclude fields from compression/hashing entirely
///
/// ## Requirements
///
/// - The `compression_info` field must be non-Option `CompressionInfo` type
/// - The `compression_info` field must be first or last field in the struct
/// - SHA256 hashing serializes the entire struct (no `#[hash]` needed)
#[proc_macro_derive(LightAccount, attributes(compress_as, skip))]
pub fn light_account_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    into_token_stream(light_pdas::account::derive::derive_light_account(input))
}

/// Derives a Rent Sponsor PDA for a program at compile time.
///
/// Seeds: ["rent_sponsor"]
///
/// ## Example
///
/// ```ignore
/// use light_sdk_macros::derive_light_rent_sponsor_pda;
///
/// pub const RENT_SPONSOR_DATA: ([u8; 32], u8) =
///     derive_light_rent_sponsor_pda!("8Ld9pGkCNfU6A7KdKe1YrTNYJWKMCFqVHqmUvjNmER7B");
/// ```
#[proc_macro]
pub fn derive_light_rent_sponsor_pda(input: TokenStream) -> TokenStream {
    rent_sponsor::derive_light_rent_sponsor_pda(input)
}

/// Derives a complete Rent Sponsor configuration for a program at compile time.
///
/// Returns ::light_sdk_types::RentSponsor { program_id, rent_sponsor, bump }.
///
/// ## Example
///
/// ```ignore
/// use light_sdk_macros::derive_light_rent_sponsor;
///
/// pub const RENT_SPONSOR: ::light_sdk_types::RentSponsor =
///     derive_light_rent_sponsor!("8Ld9pGkCNfU6A7KdKe1YrTNYJWKMCFqVHqmUvjNmER7B");
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
/// - Accounts marked with `#[light_account(token, ...)]` (rent-free token accounts)
///
/// The trait is defined in `light_account::LightFinalize`.
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
///         seeds = [b"vault", mint.key().as_ref()],
///         bump
///     )]
///     #[light_account(token, authority = [b"vault_authority"])]
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
#[proc_macro_derive(LightAccounts, attributes(light_account, instruction))]
pub fn light_accounts_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    into_token_stream(light_pdas::accounts::derive_light_accounts(input))
}
