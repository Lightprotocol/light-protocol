use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::Pubkey;
use light_compressible::compression_info::CompressionInfo;
use light_token_interface::state::{
    cmint_top_up_lamports_from_slice,
    extensions::{AdditionalMetadata, ExtensionStruct, TokenMetadata},
    BaseMint, CompressedMint, CompressedMintConfig, CompressedMintMetadata, ACCOUNT_TYPE_MINT,
};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyNew};
use rand::{thread_rng, Rng};

/// Generate random token metadata extension
fn generate_random_token_metadata(rng: &mut impl Rng, mint: Pubkey) -> TokenMetadata {
    let update_authority = if rng.gen_bool(0.7) {
        Pubkey::from(rng.gen::<[u8; 32]>())
    } else {
        Pubkey::from([0u8; 32]) // Zero pubkey for None
    };

    let name_len = rng.gen_range(1..=32);
    let name: Vec<u8> = (0..name_len).map(|_| rng.gen::<u8>()).collect();

    let symbol_len = rng.gen_range(1..=10);
    let symbol: Vec<u8> = (0..symbol_len).map(|_| rng.gen::<u8>()).collect();

    let uri_len = rng.gen_range(0..=100);
    let uri: Vec<u8> = (0..uri_len).map(|_| rng.gen::<u8>()).collect();

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

/// Generate a random CompressedMint for testing
fn generate_random_compressed_mint(rng: &mut impl Rng, with_extensions: bool) -> CompressedMint {
    let mint = Pubkey::from(rng.gen::<[u8; 32]>());

    let extensions = if with_extensions {
        let token_metadata = generate_random_token_metadata(rng, mint);
        Some(vec![ExtensionStruct::TokenMetadata(token_metadata)])
    } else {
        None
    };

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
            mint,
            cmint_decompressed: rng.gen_bool(0.5),
            mint_signer: rng.gen::<[u8; 32]>(),
            bump: rng.gen(),
        },
        reserved: [0u8; 16],
        account_type: ACCOUNT_TYPE_MINT,
        compression: CompressionInfo::default(),
        extensions,
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
        let original_mint = generate_random_compressed_mint(&mut rng, false);
        let borsh_bytes = original_mint.try_to_vec().unwrap();
        println!("Iteration {}: Borsh size = {} bytes", i, borsh_bytes.len());
        let borsh_deserialized = CompressedMint::deserialize_reader(&mut borsh_bytes.as_slice())
            .unwrap_or_else(|_| panic!("Failed to deserialize CompressedMint at iteration {}", i));
        assert_eq!(
            original_mint, borsh_deserialized,
            "Borsh roundtrip failed at iteration {}",
            i
        );

        // Test zero-copy serialization
        let config = CompressedMintConfig { extensions: None };
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
        zc_mint.base.supply = original_mint.base.supply.into();
        zc_mint.base.decimals = original_mint.base.decimals;
        zc_mint.base.is_initialized = if original_mint.base.is_initialized {
            1
        } else {
            0
        };
        zc_mint
            .base
            .set_freeze_authority(original_mint.base.freeze_authority);
        zc_mint.base.metadata.version = original_mint.metadata.version;
        zc_mint.base.metadata.mint = original_mint.metadata.mint;
        zc_mint.base.metadata.cmint_decompressed = if original_mint.metadata.cmint_decompressed {
            1
        } else {
            0
        };
        zc_mint.base.metadata.mint_signer = original_mint.metadata.mint_signer;
        zc_mint.base.metadata.bump = original_mint.metadata.bump;
        // account_type is already set in new_zero_copy
        // Set compression fields
        zc_mint.base.compression.config_account_version =
            original_mint.compression.config_account_version.into();
        zc_mint.base.compression.compress_to_pubkey = original_mint.compression.compress_to_pubkey;
        zc_mint.base.compression.account_version = original_mint.compression.account_version;
        zc_mint.base.compression.lamports_per_write =
            original_mint.compression.lamports_per_write.into();
        zc_mint.base.compression.compression_authority =
            original_mint.compression.compression_authority;
        zc_mint.base.compression.rent_sponsor = original_mint.compression.rent_sponsor;
        zc_mint.base.compression.last_claimed_slot =
            original_mint.compression.last_claimed_slot.into();
        zc_mint.base.compression.rent_config.base_rent =
            original_mint.compression.rent_config.base_rent.into();
        zc_mint.base.compression.rent_config.compression_cost = original_mint
            .compression
            .rent_config
            .compression_cost
            .into();
        zc_mint
            .base
            .compression
            .rent_config
            .lamports_per_byte_per_epoch = original_mint
            .compression
            .rent_config
            .lamports_per_byte_per_epoch;
        zc_mint.base.compression.rent_config.max_funded_epochs =
            original_mint.compression.rent_config.max_funded_epochs;
        zc_mint.base.compression.rent_config.max_top_up =
            original_mint.compression.rent_config.max_top_up.into();

        // Now deserialize the zero-copy bytes with borsh
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
        );

        // Test zero-copy read
        let (zc_read, _) = CompressedMint::zero_copy_at(&zero_copy_bytes).unwrap_or_else(|_| {
            panic!("Failed to read zero-copy CompressedMint at iteration {}", i)
        });

        // Verify fields match
        assert_eq!(
            original_mint.base.mint_authority,
            zc_read.base.mint_authority().copied(),
            "Mint authority mismatch at iteration {}",
            i
        );
        assert_eq!(
            original_mint.base.supply,
            u64::from(zc_read.base.supply),
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
            zc_read.base.freeze_authority().copied(),
            "Freeze authority mismatch at iteration {}",
            i
        );
        assert_eq!(
            original_mint.metadata.version, zc_read.base.metadata.version,
            "Version mismatch at iteration {}",
            i
        );
        assert_eq!(
            original_mint.metadata.mint, zc_read.base.metadata.mint,
            "SPL mint mismatch at iteration {}",
            i
        );
        assert_eq!(
            original_mint.metadata.cmint_decompressed,
            zc_read.base.metadata.cmint_decompressed != 0,
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
            cmint_decompressed: false,
            mint_signer: [0u8; 32],
            bump: 0,
        },
        reserved: [0u8; 16],
        account_type: ACCOUNT_TYPE_MINT,
        compression: CompressionInfo::default(),
        extensions: None,
    };

    // Borsh roundtrip
    let bytes = mint_no_auth.try_to_vec().unwrap();
    println!("Borsh serialized size: {} bytes", bytes.len());
    println!("All bytes: {:?}", &bytes);
    let deserialized = CompressedMint::deserialize(&mut bytes.as_slice()).unwrap();
    assert_eq!(mint_no_auth, deserialized);

    // Zero-copy roundtrip
    let config = CompressedMintConfig { extensions: None };

    let byte_len = CompressedMint::byte_len(&config).unwrap();
    let mut zc_bytes = vec![0u8; byte_len];
    let (mut zc_mint, _) = CompressedMint::new_zero_copy(&mut zc_bytes, config).unwrap();

    zc_mint
        .base
        .set_mint_authority(mint_no_auth.base.mint_authority);
    zc_mint.base.supply = mint_no_auth.base.supply.into();
    zc_mint.base.decimals = mint_no_auth.base.decimals;
    zc_mint.base.is_initialized = 1;
    zc_mint
        .base
        .set_freeze_authority(mint_no_auth.base.freeze_authority);
    zc_mint.base.metadata.version = mint_no_auth.metadata.version;
    zc_mint.base.metadata.mint = mint_no_auth.metadata.mint;
    zc_mint.base.metadata.cmint_decompressed = 0;
    zc_mint.base.metadata.mint_signer = mint_no_auth.metadata.mint_signer;
    zc_mint.base.metadata.bump = mint_no_auth.metadata.bump;
    // account_type is already set in new_zero_copy
    // Set compression fields
    zc_mint.base.compression.config_account_version =
        mint_no_auth.compression.config_account_version.into();
    zc_mint.base.compression.compress_to_pubkey = mint_no_auth.compression.compress_to_pubkey;
    zc_mint.base.compression.account_version = mint_no_auth.compression.account_version;
    zc_mint.base.compression.lamports_per_write =
        mint_no_auth.compression.lamports_per_write.into();
    zc_mint.base.compression.compression_authority = mint_no_auth.compression.compression_authority;
    zc_mint.base.compression.rent_sponsor = mint_no_auth.compression.rent_sponsor;
    zc_mint.base.compression.last_claimed_slot = mint_no_auth.compression.last_claimed_slot.into();
    zc_mint.base.compression.rent_config.base_rent =
        mint_no_auth.compression.rent_config.base_rent.into();
    zc_mint.base.compression.rent_config.compression_cost =
        mint_no_auth.compression.rent_config.compression_cost.into();
    zc_mint
        .base
        .compression
        .rent_config
        .lamports_per_byte_per_epoch = mint_no_auth
        .compression
        .rent_config
        .lamports_per_byte_per_epoch;
    zc_mint.base.compression.rent_config.max_funded_epochs =
        mint_no_auth.compression.rent_config.max_funded_epochs;
    zc_mint.base.compression.rent_config.max_top_up =
        mint_no_auth.compression.rent_config.max_top_up.into();

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
            cmint_decompressed: true,
            mint_signer: [0xcc; 32],
            bump: 255,
        },
        reserved: [0u8; 16],
        account_type: ACCOUNT_TYPE_MINT,
        compression: CompressionInfo::default(),
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
            cmint_decompressed: false,
            mint_signer: [4u8; 32],
            bump: 255,
        },
        reserved: [0u8; 16],
        account_type: ACCOUNT_TYPE_MINT,
        compression: CompressionInfo::default(),
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

#[test]
fn test_compressed_mint_new_zero_copy_fails_if_already_initialized() {
    let config = CompressedMintConfig { extensions: None };
    let byte_len = CompressedMint::byte_len(&config).unwrap();
    let mut buffer = vec![0u8; byte_len];

    // First initialization should succeed
    let _ = CompressedMint::new_zero_copy(&mut buffer, config.clone())
        .expect("First init should succeed");

    // Second initialization should fail because account is already initialized
    let result = CompressedMint::new_zero_copy(&mut buffer, config);
    assert!(
        result.is_err(),
        "new_zero_copy should fail if account is already initialized"
    );
    assert_eq!(
        result.unwrap_err(),
        light_zero_copy::errors::ZeroCopyError::MemoryNotZeroed
    );
}

/// Test that cmint_top_up_lamports_from_slice produces identical results to full deserialization.
#[test]
fn test_cmint_top_up_lamports_matches_full_deserialization() {
    // Create a CMint using zero-copy
    let config = CompressedMintConfig { extensions: None };
    let byte_len = CompressedMint::byte_len(&config).unwrap();
    let mut buffer = vec![0u8; byte_len];
    let (mut cmint, _) = CompressedMint::new_zero_copy(&mut buffer, config).unwrap();

    // Set known values in CompressionInfo
    cmint.base.compression.lamports_per_write = 1000.into();
    cmint.base.compression.last_claimed_slot = 13500.into(); // Epoch 1
    cmint.base.compression.rent_exemption_paid = 50_000.into();
    cmint.base.compression.rent_config.base_rent = 128.into();
    cmint.base.compression.rent_config.compression_cost = 11000.into();
    cmint
        .base
        .compression
        .rent_config
        .lamports_per_byte_per_epoch = 1;
    cmint.base.compression.rent_config.max_funded_epochs = 2;

    // Test parameters
    let current_slot = 27000u64; // Epoch 2
    let current_lamports = 100_000u64;

    // Calculate using optimized function
    let optimized_result =
        cmint_top_up_lamports_from_slice(&buffer, current_lamports, current_slot)
            .expect("Should return Some");

    // Calculate using full deserialization
    let (cmint_read, _) = CompressedMint::zero_copy_at(&buffer).unwrap();
    let full_deser_result = cmint_read
        .base
        .compression
        .calculate_top_up_lamports(buffer.len() as u64, current_slot, current_lamports)
        .expect("Should succeed");

    assert_eq!(
        optimized_result, full_deser_result,
        "Optimized result should match full deserialization"
    );
}
