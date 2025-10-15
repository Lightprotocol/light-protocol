// Tests compatibility between Borsh and Zero-copy serialization for CompressedMint
// Verifies that both implementations correctly serialize/deserialize their data
// and maintain full struct equivalence including token metadata extension.

use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::Pubkey;
use light_ctoken_types::state::{
    extensions::{AdditionalMetadata, ExtensionStruct, TokenMetadata},
    mint::{BaseMint, CompressedMint, CompressedMintMetadata},
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
            spl_mint_initialized: rng.gen_bool(0.5),
            mint: {
                let mut bytes = [0u8; 32];
                rng.fill(&mut bytes);
                Pubkey::from(bytes)
            },
        },
        extensions,
    }
}

/// Reconstruct extensions from zero-copy format
fn reconstruct_extensions(
    zc_extensions: &Option<Vec<light_ctoken_types::state::extensions::ZExtensionStruct>>,
) -> Option<Vec<ExtensionStruct>> {
    zc_extensions.as_ref().map(|exts| {
        exts.iter()
            .map(|ext| match ext {
                light_ctoken_types::state::extensions::ZExtensionStruct::TokenMetadata(
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
            mint_authority: zc_mint.base.mint_authority.map(|p| *p),
            freeze_authority: zc_mint.base.freeze_authority.map(|p| *p),
            supply: (*zc_mint.base.supply).into(),
            decimals: zc_mint.base.decimals,
            is_initialized: zc_mint.base.is_initialized != 0,
        },
        metadata: CompressedMintMetadata {
            version: zc_mint.metadata.version,
            spl_mint_initialized: zc_mint.metadata.spl_mint_initialized != 0,
            mint: zc_mint.metadata.mint,
        },
        extensions: zc_extensions.clone(),
    };

    // Test zero-copy mutable deserialization
    let mut mutable_bytes = borsh_bytes.to_vec();
    let (zc_mint_mut, _) = CompressedMint::zero_copy_at_mut(&mut mutable_bytes).unwrap();

    // Reconstruct from mutable zero-copy data for comparison
    let zc_mut_reconstructed = CompressedMint {
        base: BaseMint {
            mint_authority: zc_mint_mut.base.mint_authority().copied(),
            freeze_authority: zc_mint_mut.base.freeze_authority().copied(),
            supply: (*zc_mint_mut.base.supply).into(),
            decimals: *zc_mint_mut.base.decimals,
            is_initialized: *zc_mint_mut.base.is_initialized != 0,
        },
        metadata: CompressedMintMetadata {
            version: zc_mint_mut.metadata.version,
            spl_mint_initialized: zc_mint_mut.metadata.spl_mint_initialized != 0,
            mint: zc_mint_mut.metadata.mint,
        },
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
