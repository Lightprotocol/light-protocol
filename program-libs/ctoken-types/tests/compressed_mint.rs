use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::Pubkey;
use light_ctoken_types::state::{
    BaseMint, CompressedMint, CompressedMintConfig, CompressedMintMetadata,
};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyNew};
use rand::{thread_rng, Rng};

/// Generate a random CompressedMint for testing
fn generate_random_compressed_mint(rng: &mut impl Rng, with_extensions: bool) -> CompressedMint {
    CompressedMint {
        base: BaseMint {
            mint_authority: if rng.gen_bool(0.7) {
                Some(Pubkey::from(rng.gen::<[u8; 32]>()))
            } else {
                None
            },
            supply: rng.gen(),
            decimals: rng.gen_range(0..=18),
            is_initialized: true,
            freeze_authority: if rng.gen_bool(0.5) {
                Some(Pubkey::from(rng.gen::<[u8; 32]>()))
            } else {
                None
            },
        },
        metadata: CompressedMintMetadata {
            version: 3,
            mint: Pubkey::from(rng.gen::<[u8; 32]>()),
            spl_mint_initialized: rng.gen_bool(0.5),
        },
        extensions: if with_extensions {
            // For simplicity, we'll test without extensions for now
            // Extensions require more complex setup
            None
        } else {
            None
        },
    }
}
#[derive(BorshDeserialize, BorshSerialize, PartialEq, Debug)]
pub struct VecTestStruct {
    pub opt_vec: Option<Vec<u32>>,
}

/// Test that CompressedMint borsh serialization and zero-copy representations are compatible
#[test]
fn test_compressed_mint_borsh_zerocopy_compatibility() {
    let test = VecTestStruct { opt_vec: None };
    let test_bytes = test.try_to_vec().unwrap();
    println!("test bytes {:?}", test_bytes);
    let deserialize = VecTestStruct::deserialize(&mut test_bytes.as_slice()).unwrap();
    assert_eq!(test, deserialize);

    let mut rng = thread_rng();

    for i in 0..100 {
        let original_mint = generate_random_compressed_mint(&mut rng, false); // Test Borsh serialization roundtrip
        let borsh_bytes = original_mint.try_to_vec().unwrap();
        println!("Iteration {}: Borsh size = {} bytes", i, borsh_bytes.len());
        let borsh_deserialized = CompressedMint::deserialize_reader(&mut borsh_bytes.as_slice())
            .unwrap_or_else(|_| panic!("Failed to deserialize CompressedMint at iteration {}", i));
        assert_eq!(
            original_mint, borsh_deserialized,
            "Borsh roundtrip failed at iteration {}",
            i
        ); // Test zero-copy serialization
        let config = CompressedMintConfig {
            base: (),
            metadata: (),
            extensions: (false, vec![]),
        };
        let byte_len = CompressedMint::byte_len(&config).unwrap();
        let mut zero_copy_bytes = vec![0u8; byte_len];
        let (mut zc_mint, _) = CompressedMint::new_zero_copy(&mut zero_copy_bytes, config)
            .unwrap_or_else(|_| {
                panic!(
                    "Failed to create zero-copy CompressedMint at iteration {}",
                    i
                )
            });
        // Set the zero-copy fields to match original
        zc_mint
            .base
            .set_mint_authority(original_mint.base.mint_authority);
        *zc_mint.base.supply = original_mint.base.supply.into();
        *zc_mint.base.decimals = original_mint.base.decimals;
        *zc_mint.base.is_initialized = if original_mint.base.is_initialized {
            1
        } else {
            0
        };
        zc_mint
            .base
            .set_freeze_authority(original_mint.base.freeze_authority);
        zc_mint.metadata.version = original_mint.metadata.version;
        zc_mint.metadata.mint = original_mint.metadata.mint;
        zc_mint.metadata.spl_mint_initialized = if original_mint.metadata.spl_mint_initialized {
            1
        } else {
            0
        }; // Now deserialize the zero-copy bytes with borsh
        let zc_as_borsh = CompressedMint::deserialize(&mut zero_copy_bytes.as_slice())
            .unwrap_or_else(|_| {
                panic!(
                    "Failed to deserialize zero-copy bytes as borsh at iteration {}",
                    i
                )
            });
        assert_eq!(
            original_mint, zc_as_borsh,
            "Zero-copy to borsh conversion failed at iteration {}",
            i
        ); // Test zero-copy read
        let (zc_read, _) = CompressedMint::zero_copy_at(&zero_copy_bytes).unwrap_or_else(|_| {
            panic!("Failed to read zero-copy CompressedMint at iteration {}", i)
        });
        // Verify fields match
        assert_eq!(
            original_mint.base.mint_authority,
            zc_read.base.mint_authority.map(|a| *a),
            "Mint authority mismatch at iteration {}",
            i
        );
        assert_eq!(
            original_mint.base.supply,
            u64::from(*zc_read.base.supply),
            "Supply mismatch at iteration {}",
            i
        );
        assert_eq!(
            original_mint.base.decimals, zc_read.base.decimals,
            "Decimals mismatch at iteration {}",
            i
        );
        assert_eq!(
            original_mint.base.freeze_authority,
            zc_read.base.freeze_authority.map(|a| *a),
            "Freeze authority mismatch at iteration {}",
            i
        );
        assert_eq!(
            original_mint.metadata.version, zc_read.metadata.version,
            "Version mismatch at iteration {}",
            i
        );
        assert_eq!(
            original_mint.metadata.mint, zc_read.metadata.mint,
            "SPL mint mismatch at iteration {}",
            i
        );
        assert_eq!(
            original_mint.metadata.spl_mint_initialized,
            zc_read.metadata.spl_mint_initialized != 0,
            "Is decompressed mismatch at iteration {}",
            i
        );
    }
}

/// Test edge cases for CompressedMint serialization
#[test]
fn test_compressed_mint_edge_cases() {
    // Test with no authorities
    let mint_no_auth = CompressedMint {
        base: BaseMint {
            mint_authority: None,
            supply: u64::MAX,
            decimals: 0,
            is_initialized: true,
            freeze_authority: None,
        },
        metadata: CompressedMintMetadata {
            version: 3,
            mint: Pubkey::from([0xff; 32]),
            spl_mint_initialized: false,
        },
        extensions: None,
    };

    // Borsh roundtrip
    let bytes = mint_no_auth.try_to_vec().unwrap();
    println!("Borsh serialized size: {} bytes", bytes.len());
    println!("All bytes: {:?}", &bytes);
    let deserialized = CompressedMint::deserialize(&mut bytes.as_slice()).unwrap();
    assert_eq!(mint_no_auth, deserialized);

    // Zero-copy roundtrip
    let config = CompressedMintConfig {
        base: (),
        metadata: (),
        extensions: (false, vec![]),
    };

    let byte_len = CompressedMint::byte_len(&config).unwrap();
    let mut zc_bytes = vec![0u8; byte_len];
    let (mut zc_mint, _) = CompressedMint::new_zero_copy(&mut zc_bytes, config).unwrap();

    zc_mint
        .base
        .set_mint_authority(mint_no_auth.base.mint_authority);
    *zc_mint.base.supply = mint_no_auth.base.supply.into();
    *zc_mint.base.decimals = mint_no_auth.base.decimals;
    *zc_mint.base.is_initialized = 1;
    zc_mint
        .base
        .set_freeze_authority(mint_no_auth.base.freeze_authority);
    zc_mint.metadata.version = mint_no_auth.metadata.version;
    zc_mint.metadata.mint = mint_no_auth.metadata.mint;
    zc_mint.metadata.spl_mint_initialized = 0;

    let zc_as_borsh = CompressedMint::deserialize(&mut zc_bytes.as_slice()).unwrap();
    assert_eq!(mint_no_auth, zc_as_borsh);

    // Test with maximum values
    let mint_max = CompressedMint {
        base: BaseMint {
            mint_authority: Some(Pubkey::from([0xff; 32])),
            supply: u64::MAX,
            decimals: 255,
            is_initialized: true,
            freeze_authority: Some(Pubkey::from([0xaa; 32])),
        },
        metadata: CompressedMintMetadata {
            version: 255,
            mint: Pubkey::from([0xbb; 32]),
            spl_mint_initialized: true,
        },
        extensions: None,
    };

    let bytes = mint_max.try_to_vec().unwrap();
    let deserialized = CompressedMint::deserialize(&mut bytes.as_slice()).unwrap();
    assert_eq!(mint_max, deserialized);
}

/// Test that BaseMint within CompressedMint maintains SPL compatibility format
#[test]
fn test_base_mint_in_compressed_mint_spl_format() {
    let mint = CompressedMint {
        base: BaseMint {
            mint_authority: Some(Pubkey::from([1; 32])),
            supply: 1000000,
            decimals: 9,
            is_initialized: true,
            freeze_authority: Some(Pubkey::from([2; 32])),
        },
        metadata: CompressedMintMetadata {
            version: 3,
            mint: Pubkey::from([3; 32]),
            spl_mint_initialized: false,
        },
        extensions: None,
    };

    // Serialize the whole CompressedMint
    let full_bytes = mint.try_to_vec().unwrap();

    // The BaseMint portion should be at the beginning
    // and should be 82 bytes (SPL Mint size)
    assert!(
        full_bytes.len() >= 82,
        "Serialized CompressedMint should be at least 82 bytes"
    );

    // Extract just the BaseMint portion
    let base_mint_bytes = &full_bytes[..82];

    // Deserialize as BaseMint to verify format
    let base_mint = BaseMint::deserialize(&mut base_mint_bytes.to_vec().as_slice()).unwrap();
    assert_eq!(mint.base, base_mint);
}
