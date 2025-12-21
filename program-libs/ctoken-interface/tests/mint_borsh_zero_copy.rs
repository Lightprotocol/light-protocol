// Tests compatibility between Borsh and Zero-copy serialization for CompressedMint
// Verifies that both implementations correctly serialize/deserialize their data
// and maintain full struct equivalence including token metadata extension.

use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::Pubkey;
use light_compressible::compression_info::CompressionInfo;
use light_ctoken_interface::state::{
    extensions::{AdditionalMetadata, ExtensionStruct, TokenMetadata},
    mint::{BaseMint, CompressedMint, CompressedMintMetadata, ACCOUNT_TYPE_MINT},
};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use rand::{thread_rng, Rng};
use spl_token_2022::{solana_program::program_pack::Pack, state::Mint};

/// Generate random token metadata extension
fn generate_random_token_metadata(rng: &mut impl Rng) -> TokenMetadata {
    // Random update authority
    let update_authority = if rng.gen_bool(0.7) {
        let mut bytes = [0u8; 32];
        rng.fill(&mut bytes);
        Pubkey::from(bytes)
    } else {
        Pubkey::from([0u8; 32]) // Zero pubkey for None
    };

    // Random mint
    let mut mint_bytes = [0u8; 32];
    rng.fill(&mut mint_bytes);
    let mint = Pubkey::from(mint_bytes);

    // Random name (1-32 chars)
    let name_len = rng.gen_range(1..=32);
    let name: Vec<u8> = (0..name_len).map(|_| rng.gen::<u8>()).collect();

    // Random symbol (1-10 chars)
    let symbol_len = rng.gen_range(1..=10);
    let symbol: Vec<u8> = (0..symbol_len).map(|_| rng.gen::<u8>()).collect();

    // Random URI (0-200 chars)
    let uri_len = rng.gen_range(0..=200);
    let uri: Vec<u8> = (0..uri_len).map(|_| rng.gen::<u8>()).collect();

    // Random additional metadata (0-3 entries)
    let num_metadata = rng.gen_range(0..=3);
    let additional_metadata: Vec<AdditionalMetadata> = (0..num_metadata)
        .map(|_| {
            let key_len = rng.gen_range(1..=20);
            let key: Vec<u8> = (0..key_len).map(|_| rng.gen::<u8>()).collect();
            let value_len = rng.gen_range(0..=50);
            let value: Vec<u8> = (0..value_len).map(|_| rng.gen::<u8>()).collect();
            AdditionalMetadata { key, value }
        })
        .collect();

    TokenMetadata {
        update_authority,
        mint,
        name,
        symbol,
        uri,
        additional_metadata,
    }
}

/// Generate random CompressedMint for testing
fn generate_random_mint() -> CompressedMint {
    let mut rng = thread_rng();

    // 40% chance to include token metadata extension
    let extensions = if rng.gen_bool(0.4) {
        let token_metadata = generate_random_token_metadata(&mut rng);
        Some(vec![ExtensionStruct::TokenMetadata(token_metadata)])
    } else {
        None
    };

    CompressedMint {
        base: BaseMint {
            mint_authority: if rng.gen_bool(0.7) {
                let mut bytes = [0u8; 32];
                rng.fill(&mut bytes);
                Some(Pubkey::from(bytes))
            } else {
                None
            },
            freeze_authority: if rng.gen_bool(0.5) {
                let mut bytes = [0u8; 32];
                rng.fill(&mut bytes);
                Some(Pubkey::from(bytes))
            } else {
                None
            },
            supply: rng.gen::<u64>(),
            decimals: rng.gen_range(0..=18),
            is_initialized: true,
        },
        metadata: CompressedMintMetadata {
            version: 3,
            cmint_decompressed: rng.gen_bool(0.5),
            mint: {
                let mut bytes = [0u8; 32];
                rng.fill(&mut bytes);
                Pubkey::from(bytes)
            },
        },
        reserved: [0u8; 49],
        account_type: ACCOUNT_TYPE_MINT,
        compression: CompressionInfo::default(),
        extensions,
    }
}

/// Reconstruct extensions from zero-copy format
fn reconstruct_extensions(
    zc_extensions: &Option<Vec<light_ctoken_interface::state::extensions::ZExtensionStruct>>,
) -> Option<Vec<ExtensionStruct>> {
    zc_extensions.as_ref().map(|exts| {
        exts.iter()
            .map(|ext| match ext {
                light_ctoken_interface::state::extensions::ZExtensionStruct::TokenMetadata(
                    zc_metadata,
                ) => ExtensionStruct::TokenMetadata(TokenMetadata {
                    update_authority: zc_metadata.update_authority,
                    mint: zc_metadata.mint,
                    name: zc_metadata.name.to_vec(),
                    symbol: zc_metadata.symbol.to_vec(),
                    uri: zc_metadata.uri.to_vec(),
                    additional_metadata: zc_metadata
                        .additional_metadata
                        .iter()
                        .map(|am| AdditionalMetadata {
                            key: am.key.to_vec(),
                            value: am.value.to_vec(),
                        })
                        .collect(),
                }),
                _ => panic!("Unexpected extension type in test"),
            })
            .collect()
    })
}

/// Compare Borsh-serialized mint with zero-copy deserialized versions
fn compare_mint_borsh_vs_zero_copy(original: &CompressedMint, borsh_bytes: &[u8]) {
    // Deserialize using Borsh
    let borsh_mint = CompressedMint::try_from_slice(borsh_bytes).unwrap();

    // Deserialize using zero-copy (read-only)
    let (zc_mint, _) = CompressedMint::zero_copy_at(borsh_bytes).unwrap();

    // Reconstruct extensions from zero-copy format
    let zc_extensions = reconstruct_extensions(&zc_mint.extensions);

    // Construct a CompressedMint from zero-copy read-only data for comparison
    let zc_reconstructed = CompressedMint {
        base: BaseMint {
            mint_authority: zc_mint.meta.mint_authority().copied(),
            freeze_authority: zc_mint.meta.freeze_authority().copied(),
            supply: u64::from(zc_mint.meta.supply),
            decimals: zc_mint.meta.decimals,
            is_initialized: zc_mint.meta.is_initialized != 0,
        },
        metadata: CompressedMintMetadata {
            version: zc_mint.meta.metadata.version,
            cmint_decompressed: zc_mint.meta.metadata.cmint_decompressed != 0,
            mint: zc_mint.meta.metadata.mint,
        },
        reserved: *zc_mint.meta.reserved,
        account_type: zc_mint.meta.account_type,
        compression: CompressionInfo::default(),
        extensions: zc_extensions.clone(),
    };

    // Test zero-copy mutable deserialization
    let mut mutable_bytes = borsh_bytes.to_vec();
    let (zc_mint_mut, _) = CompressedMint::zero_copy_at_mut(&mut mutable_bytes).unwrap();

    // Reconstruct from mutable zero-copy data for comparison
    let zc_mut_reconstructed = CompressedMint {
        base: BaseMint {
            mint_authority: zc_mint_mut.meta.mint_authority().copied(),
            freeze_authority: zc_mint_mut.meta.freeze_authority().copied(),
            supply: u64::from(zc_mint_mut.meta.supply),
            decimals: zc_mint_mut.meta.decimals,
            is_initialized: zc_mint_mut.meta.is_initialized != 0,
        },
        metadata: CompressedMintMetadata {
            version: zc_mint_mut.meta.metadata.version,
            cmint_decompressed: zc_mint_mut.meta.metadata.cmint_decompressed != 0,
            mint: zc_mint_mut.meta.metadata.mint,
        },
        reserved: *zc_mint_mut.meta.reserved,
        account_type: *zc_mint_mut.meta.account_type,
        compression: CompressionInfo::default(),
        extensions: zc_extensions, // Extensions handling for mut is same as read-only
    };

    // Single assertion comparing all four structs
    assert_eq!(
        (original, &borsh_mint, &zc_reconstructed, &zc_mut_reconstructed),
        (original, original, original, original),
        "Mismatch between original, Borsh, zero-copy read-only, and zero-copy mutable deserialized structs"
    );

    // Test SPL mint pod deserialization on base mint only
    // Only use the first Mint::LEN bytes for SPL deserialization
    let mint = Mint::unpack(&borsh_bytes[..Mint::LEN]).unwrap();

    // Reconstruct BaseMint from SPL mint for comparison
    let spl_reconstructed_base = BaseMint {
        mint_authority: Option::<solana_pubkey::Pubkey>::from(mint.mint_authority)
            .map(|p| Pubkey::from(p.to_bytes())),
        freeze_authority: Option::<solana_pubkey::Pubkey>::from(mint.freeze_authority)
            .map(|p| Pubkey::from(p.to_bytes())),
        supply: mint.supply,
        decimals: mint.decimals,
        is_initialized: mint.is_initialized,
    };

    // Additional assertion comparing base mints with SPL pod deserialization
    assert_eq!(
        &original.base, &spl_reconstructed_base,
        "Mismatch between original base mint and SPL pod deserialized base mint"
    );
}

/// Randomized test comparing Borsh and zero-copy serialization (1k iterations)
#[test]
fn test_mint_borsh_zero_copy_compatibility() {
    for _ in 0..1000 {
        let mint = generate_random_mint();
        let borsh_bytes = mint.try_to_vec().unwrap();
        compare_mint_borsh_vs_zero_copy(&mint, &borsh_bytes);
    }
}

/// Generate mint with guaranteed TokenMetadata extension
fn generate_mint_with_extensions() -> CompressedMint {
    let mut rng = thread_rng();
    let token_metadata = generate_random_token_metadata(&mut rng);

    CompressedMint {
        base: BaseMint {
            mint_authority: Some(Pubkey::from(rng.gen::<[u8; 32]>())),
            freeze_authority: Some(Pubkey::from(rng.gen::<[u8; 32]>())),
            supply: rng.gen::<u64>(),
            decimals: rng.gen_range(0..=18),
            is_initialized: true,
        },
        metadata: CompressedMintMetadata {
            version: 3,
            cmint_decompressed: rng.gen_bool(0.5),
            mint: Pubkey::from(rng.gen::<[u8; 32]>()),
        },
        reserved: [0u8; 49],
        account_type: ACCOUNT_TYPE_MINT,
        compression: CompressionInfo::default(),
        extensions: Some(vec![ExtensionStruct::TokenMetadata(token_metadata)]),
    }
}

/// Test with guaranteed extensions - ensures extension path is always tested
#[test]
fn test_mint_with_extensions_borsh_zero_copy_compatibility() {
    for _ in 0..500 {
        let mint = generate_mint_with_extensions();
        let borsh_bytes = mint.try_to_vec().unwrap();
        compare_mint_borsh_vs_zero_copy(&mint, &borsh_bytes);
    }
}

/// Test extension edge cases
#[test]
fn test_mint_extension_edge_cases() {
    // Test 1: Empty strings in TokenMetadata
    let mint_empty_strings = CompressedMint {
        base: BaseMint {
            mint_authority: Some(Pubkey::from([1u8; 32])),
            freeze_authority: None,
            supply: 1_000_000,
            decimals: 9,
            is_initialized: true,
        },
        metadata: CompressedMintMetadata {
            version: 3,
            cmint_decompressed: false,
            mint: Pubkey::from([2u8; 32]),
        },
        reserved: [0u8; 49],
        account_type: ACCOUNT_TYPE_MINT,
        compression: CompressionInfo::default(),
        extensions: Some(vec![ExtensionStruct::TokenMetadata(TokenMetadata {
            update_authority: Pubkey::from([3u8; 32]),
            mint: Pubkey::from([2u8; 32]),
            name: vec![],           // Empty name
            symbol: vec![],         // Empty symbol
            uri: vec![],            // Empty URI
            additional_metadata: vec![], // No additional metadata
        })]),
    };
    let borsh_bytes = mint_empty_strings.try_to_vec().unwrap();
    compare_mint_borsh_vs_zero_copy(&mint_empty_strings, &borsh_bytes);

    // Test 2: Maximum reasonable lengths
    let mint_max_lengths = CompressedMint {
        base: BaseMint {
            mint_authority: Some(Pubkey::from([0xffu8; 32])),
            freeze_authority: Some(Pubkey::from([0xaau8; 32])),
            supply: u64::MAX,
            decimals: 18,
            is_initialized: true,
        },
        metadata: CompressedMintMetadata {
            version: 3,
            cmint_decompressed: true,
            mint: Pubkey::from([0xbbu8; 32]),
        },
        reserved: [0u8; 49],
        account_type: ACCOUNT_TYPE_MINT,
        compression: CompressionInfo::default(),
        extensions: Some(vec![ExtensionStruct::TokenMetadata(TokenMetadata {
            update_authority: Pubkey::from([0xccu8; 32]),
            mint: Pubkey::from([0xbbu8; 32]),
            name: vec![b'A'; 64],   // Long name
            symbol: vec![b'S'; 16], // Long symbol
            uri: vec![b'U'; 256],   // Long URI
            additional_metadata: vec![
                AdditionalMetadata {
                    key: vec![b'K'; 32],
                    value: vec![b'V'; 128],
                },
                AdditionalMetadata {
                    key: vec![b'X'; 32],
                    value: vec![b'Y'; 128],
                },
                AdditionalMetadata {
                    key: vec![b'Z'; 32],
                    value: vec![b'W'; 128],
                },
            ],
        })]),
    };
    let borsh_bytes = mint_max_lengths.try_to_vec().unwrap();
    compare_mint_borsh_vs_zero_copy(&mint_max_lengths, &borsh_bytes);

    // Test 3: Zero update authority (represents None)
    let mint_zero_authority = CompressedMint {
        base: BaseMint {
            mint_authority: None,
            freeze_authority: None,
            supply: 0,
            decimals: 0,
            is_initialized: true,
        },
        metadata: CompressedMintMetadata {
            version: 3,
            cmint_decompressed: false,
            mint: Pubkey::from([4u8; 32]),
        },
        reserved: [0u8; 49],
        account_type: ACCOUNT_TYPE_MINT,
        compression: CompressionInfo::default(),
        extensions: Some(vec![ExtensionStruct::TokenMetadata(TokenMetadata {
            update_authority: Pubkey::from([0u8; 32]), // Zero = None
            mint: Pubkey::from([4u8; 32]),
            name: b"Test Token".to_vec(),
            symbol: b"TEST".to_vec(),
            uri: b"https://example.com/token.json".to_vec(),
            additional_metadata: vec![],
        })]),
    };
    let borsh_bytes = mint_zero_authority.try_to_vec().unwrap();
    compare_mint_borsh_vs_zero_copy(&mint_zero_authority, &borsh_bytes);

    // Test 4: No extensions (explicit None)
    let mint_no_extensions = CompressedMint {
        base: BaseMint {
            mint_authority: Some(Pubkey::from([5u8; 32])),
            freeze_authority: Some(Pubkey::from([6u8; 32])),
            supply: 500_000,
            decimals: 6,
            is_initialized: true,
        },
        metadata: CompressedMintMetadata {
            version: 3,
            cmint_decompressed: true,
            mint: Pubkey::from([7u8; 32]),
        },
        reserved: [0u8; 49],
        account_type: ACCOUNT_TYPE_MINT,
        compression: CompressionInfo::default(),
        extensions: None,
    };
    let borsh_bytes = mint_no_extensions.try_to_vec().unwrap();
    compare_mint_borsh_vs_zero_copy(&mint_no_extensions, &borsh_bytes);
}
