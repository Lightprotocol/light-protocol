//! Unit tests for LightCompressible derive macro.
//!
//! Extracted from `light_pdas/account/light_compressible.rs`.

use syn::{parse_quote, DeriveInput};

use crate::light_pdas::account::light_compressible::derive_light_account;

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

    let result = derive_light_account(input);
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

    let result = derive_light_account(input);
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

    let result = derive_light_account(input);
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

    let result = derive_light_account(input);
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

    let result = derive_light_account(input);
    // Compressible derive validates compression_info field
    assert!(
        result.is_err(),
        "Should fail without compression_info field"
    );
}
