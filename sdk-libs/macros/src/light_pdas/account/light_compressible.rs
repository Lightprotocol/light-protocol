//! LightCompressible derive macro - consolidates all required traits for compressible accounts.
//!
//! This macro is equivalent to deriving:
//! - `LightHasherSha` (SHA256 hashing)
//! - `LightDiscriminator` (unique discriminator)
//! - `Compressible` (HasCompressionInfo + CompressAs + Size + CompressedInitSpace)
//! - `CompressiblePack` (Pack + Unpack + Packed struct generation)

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Fields, ItemStruct, Result};

use crate::{
    discriminator::discriminator,
    hasher::derive_light_hasher_sha,
    light_pdas::account::{pack_unpack::derive_compressible_pack, traits::derive_compressible},
};

/// Derives all required traits for a compressible account.
///
/// This is a convenience macro that combines:
/// - `LightHasherSha` - SHA256-based DataHasher and ToByteArray implementations (type 3 ShaFlat)
/// - `LightDiscriminator` - Unique 8-byte discriminator for the account type
/// - `Compressible` - HasCompressionInfo, CompressAs, Size, CompressedInitSpace traits
/// - `CompressiblePack` - Pack/Unpack traits with Packed struct generation for Pubkey compression
///
/// # Example
///
/// ```ignore
/// use light_sdk_macros::LightCompressible;
/// use light_sdk::interface::CompressionInfo;
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
/// ## Notes
///
/// - The `compression_info` field is auto-detected and handled specially (no `#[skip]` needed)
/// - SHA256 hashing serializes the entire struct, so `#[hash]` is not needed
pub fn derive_light_account(input: DeriveInput) -> Result<TokenStream> {
    // Convert DeriveInput to ItemStruct for macros that need it
    let item_struct = derive_input_to_item_struct(&input)?;

    // Generate LightHasherSha implementation
    let hasher_impl = derive_light_hasher_sha(item_struct.clone())?;

    // Generate LightDiscriminator implementation
    let discriminator_impl = discriminator(item_struct)?;

    // Generate Compressible implementation (HasCompressionInfo + CompressAs + Size + CompressedInitSpace)
    let compressible_impl = derive_compressible(input.clone())?;

    // Generate CompressiblePack implementation (Pack + Unpack + Packed struct)
    let pack_impl = derive_compressible_pack(input)?;

    // Combine all implementations
    Ok(quote! {
        #hasher_impl
        #discriminator_impl
        #compressible_impl
        #pack_impl
    })
}

/// Converts a DeriveInput to an ItemStruct.
///
/// This is needed because some of our existing macros (like LightHasherSha)
/// expect ItemStruct while others (like Compressible) expect DeriveInput.
fn derive_input_to_item_struct(input: &DeriveInput) -> Result<ItemStruct> {
    let data = match &input.data {
        syn::Data::Struct(data) => data,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "LightCompressible can only be derived for structs",
            ))
        }
    };

    let fields = match &data.fields {
        Fields::Named(fields) => Fields::Named(fields.clone()),
        Fields::Unnamed(fields) => Fields::Unnamed(fields.clone()),
        Fields::Unit => Fields::Unit,
    };

    Ok(ItemStruct {
        attrs: input.attrs.clone(),
        vis: input.vis.clone(),
        struct_token: data.struct_token,
        ident: input.ident.clone(),
        generics: input.generics.clone(),
        fields,
        semi_token: data.semi_token,
    })
}
