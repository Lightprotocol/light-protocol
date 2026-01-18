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
    rentfree::account::{pack_unpack::derive_compressible_pack, traits::derive_compressible},
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
/// ## Notes
///
/// - The `compression_info` field is auto-detected and handled specially (no `#[skip]` needed)
/// - SHA256 hashing serializes the entire struct, so `#[hash]` is not needed
pub fn derive_rentfree_account(input: DeriveInput) -> Result<TokenStream> {
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

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_light_compressible_basic() {
        // No #[hash] or #[skip] needed - SHA256 hashes entire struct, compression_info auto-skipped
        let input: DeriveInput = parse_quote! {
            pub struct UserRecord {
                pub owner: Pubkey,
                pub name: String,
                pub score: u64,
                pub compression_info: Option<CompressionInfo>,
            }
        };

        let result = derive_rentfree_account(input);
        assert!(result.is_ok(), "LightCompressible should succeed");

        let output = result.unwrap().to_string();

        // Should contain LightHasherSha output
        assert!(output.contains("DataHasher"), "Should implement DataHasher");
        assert!(
            output.contains("ToByteArray"),
            "Should implement ToByteArray"
        );

        // Should contain LightDiscriminator output
        assert!(
            output.contains("LightDiscriminator"),
            "Should implement LightDiscriminator"
        );
        assert!(
            output.contains("LIGHT_DISCRIMINATOR"),
            "Should have discriminator constant"
        );

        // Should contain Compressible output (HasCompressionInfo, CompressAs, Size)
        assert!(
            output.contains("HasCompressionInfo"),
            "Should implement HasCompressionInfo"
        );
        assert!(output.contains("CompressAs"), "Should implement CompressAs");
        assert!(output.contains("Size"), "Should implement Size");

        // Should contain CompressiblePack output (Pack, Unpack, Packed struct)
        assert!(output.contains("Pack"), "Should implement Pack");
        assert!(output.contains("Unpack"), "Should implement Unpack");
        assert!(
            output.contains("PackedUserRecord"),
            "Should generate Packed struct"
        );
    }

    #[test]
    fn test_light_compressible_with_compress_as() {
        // compress_as still works - no #[hash] or #[skip] needed
        let input: DeriveInput = parse_quote! {
            #[compress_as(start_time = 0, score = 0)]
            pub struct GameSession {
                pub session_id: u64,
                pub player: Pubkey,
                pub start_time: u64,
                pub score: u64,
                pub compression_info: Option<CompressionInfo>,
            }
        };

        let result = derive_rentfree_account(input);
        assert!(
            result.is_ok(),
            "LightCompressible with compress_as should succeed"
        );

        let output = result.unwrap().to_string();

        // compress_as attribute should be processed
        assert!(output.contains("CompressAs"), "Should implement CompressAs");
    }

    #[test]
    fn test_light_compressible_no_pubkey_fields() {
        let input: DeriveInput = parse_quote! {
            pub struct SimpleRecord {
                pub id: u64,
                pub value: u32,
                pub compression_info: Option<CompressionInfo>,
            }
        };

        let result = derive_rentfree_account(input);
        assert!(
            result.is_ok(),
            "LightCompressible without Pubkey fields should succeed"
        );

        let output = result.unwrap().to_string();

        // Should still generate everything
        assert!(output.contains("DataHasher"), "Should implement DataHasher");
        assert!(
            output.contains("LightDiscriminator"),
            "Should implement LightDiscriminator"
        );
        assert!(
            output.contains("HasCompressionInfo"),
            "Should implement HasCompressionInfo"
        );

        // For structs without Pubkey fields, PackedSimpleRecord should be a type alias
        // (implementation detail of CompressiblePack)
    }

    #[test]
    fn test_light_compressible_enum_fails() {
        let input: DeriveInput = parse_quote! {
            pub enum NotAStruct {
                A,
                B,
            }
        };

        let result = derive_rentfree_account(input);
        assert!(result.is_err(), "LightCompressible should fail for enums");
    }

    #[test]
    fn test_light_compressible_missing_compression_info() {
        let input: DeriveInput = parse_quote! {
            pub struct MissingCompressionInfo {
                pub id: u64,
                pub value: u32,
            }
        };

        let result = derive_rentfree_account(input);
        // Compressible derive validates compression_info field
        assert!(
            result.is_err(),
            "Should fail without compression_info field"
        );
    }
}
