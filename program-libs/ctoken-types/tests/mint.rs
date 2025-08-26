use light_compressed_account::Pubkey;
use light_ctoken_types::{
    hash_cache::HashCache,
    state::{
        extensions::{ExtensionStruct, Metadata, TokenMetadata},
        mint::CompressedMint,
    },
};
use light_hasher::{Hasher, Poseidon};

#[test]
fn test_compressed_mint_hash_manual_verification() {
    // Hardcoded test values
    let spl_mint = Pubkey::new_from_array([1u8; 32]);
    let supply = 1000000u64; // 1M tokens
    let decimals = 6u8;
    let mint_authority = Some(Pubkey::new_from_array([2u8; 32]));
    let freeze_authority = Some(Pubkey::new_from_array([3u8; 32]));
    let version = 0u8; // Poseidon version
    let is_decompressed = true;
    let extensions: Option<Vec<ExtensionStruct>> = None;

    // Create CompressedMint with hardcoded values
    let compressed_mint = CompressedMint {
        spl_mint,
        supply,
        decimals,
        is_decompressed,
        mint_authority,
        freeze_authority,
        version,
        extensions,
    };

    // Calculate hash using CompressedMint::hash()
    let calculated_hash = compressed_mint.hash().unwrap();

    // Manual hash calculation to verify correctness
    let mut hash_cache = HashCache::new();

    // 1. Hash the spl_mint
    let hashed_spl_mint = hash_cache.get_or_hash_mint(&spl_mint.into()).unwrap();

    // 2. Convert supply to 32-byte array (big-endian, right-aligned)
    let mut supply_bytes = [0u8; 32];
    supply_bytes[24..].copy_from_slice(&supply.to_be_bytes());

    // 3. Hash authorities
    let hashed_mint_authority = hash_cache.get_or_hash_pubkey(&mint_authority.unwrap().to_bytes());
    let hashed_freeze_authority =
        hash_cache.get_or_hash_pubkey(&freeze_authority.unwrap().to_bytes());

    // 4. Calculate manual hash using the exact same format as hash_with_hashed_values_inner
    let mut hash_inputs: Vec<&[u8]> = Vec::new();

    // Add spl_mint hash
    hash_inputs.push(hashed_spl_mint.as_slice());

    // Add supply bytes
    hash_inputs.push(supply_bytes.as_slice());

    // Add decimals with prefix if not 0 (our decimals = 6, so add it)
    let mut decimals_bytes = [0u8; 32];
    if decimals != 0 {
        decimals_bytes[30] = 1; // decimals prefix
        decimals_bytes[31] = decimals;
        hash_inputs.push(&decimals_bytes[..]);
    }

    // Add is_decompressed with prefix if true (our is_decompressed = false, so skip)
    let mut is_decompressed_bytes = [0u8; 32];
    if is_decompressed {
        is_decompressed_bytes[30] = 2; // is_decompressed prefix
        is_decompressed_bytes[31] = 1; // true as 1
        hash_inputs.push(&is_decompressed_bytes[..]);
    }

    // Add mint authority if present
    hash_inputs.push(hashed_mint_authority.as_slice());

    // Add freeze authority if present
    hash_inputs.push(hashed_freeze_authority.as_slice());

    // Add version with prefix if not 0 (our version = 0, so skip)
    // let mut num_extensions_bytes = [0u8; 32];
    // if version != 0 {
    //     num_extensions_bytes[30] = 3; // version prefix
    //     num_extensions_bytes[31] = version;
    //     hash_inputs.push(&num_extensions_bytes[..]);
    // }

    let manual_hash = Poseidon::hashv(&hash_inputs).unwrap();

    // Verify that calculated hash matches manual hash
    assert_eq!(
        calculated_hash, manual_hash,
        "CompressedMint::hash() should match manual hash calculation"
    );
    let reference = [
        0, 43, 27, 117, 9, 143, 251, 145, 134, 134, 205, 7, 60, 249, 199, 156, 205, 184, 208, 10,
        52, 248, 20, 204, 176, 198, 65, 112, 135, 44, 136, 42,
    ];
    assert_eq!(calculated_hash, reference);
}

#[test]
fn test_compressed_mint_hash_with_extension_manual_verification() {
    // Hardcoded test values
    let spl_mint = Pubkey::new_from_array([1u8; 32]);
    let supply = 2000000u64; // 2M tokens
    let decimals = 8u8;
    let mint_authority = Some(Pubkey::new_from_array([10u8; 32]));
    let freeze_authority = Some(Pubkey::new_from_array([20u8; 32]));
    let version = 0u8; // Poseidon version
    let is_decompressed = false;

    // Create TokenMetadata extension
    let token_metadata = TokenMetadata {
        update_authority: Some(Pubkey::new_from_array([5u8; 32])),
        mint: spl_mint,
        metadata: Metadata {
            name: b"Test Token".to_vec(),
            symbol: b"TEST".to_vec(),
            uri: b"https://example.com/token.json".to_vec(),
        },
        additional_metadata: vec![],
        version: 0, // Poseidon
    };

    let extensions = Some(vec![ExtensionStruct::TokenMetadata(token_metadata.clone())]);

    // Create CompressedMint with extension
    let compressed_mint = CompressedMint {
        spl_mint,
        supply,
        decimals,
        is_decompressed,
        mint_authority,
        freeze_authority,
        version,
        extensions,
    };

    // Calculate hash using CompressedMint::hash()
    let calculated_hash = compressed_mint.hash().unwrap();

    // Manual hash calculation to verify correctness
    let mut hash_cache = HashCache::new();

    // 1. Hash the spl_mint
    let hashed_spl_mint = hash_cache.get_or_hash_mint(&spl_mint.into()).unwrap();

    // 2. Convert supply to 32-byte array (big-endian, right-aligned)
    let mut supply_bytes = [0u8; 32];
    supply_bytes[24..].copy_from_slice(&supply.to_be_bytes());

    // 3. Hash authorities
    let hashed_mint_authority = hash_cache.get_or_hash_pubkey(&mint_authority.unwrap().to_bytes());
    let hashed_freeze_authority =
        hash_cache.get_or_hash_pubkey(&freeze_authority.unwrap().to_bytes());

    // 4. Calculate base mint hash using the same format as hash_with_hashed_values_inner
    let mut hash_inputs: Vec<&[u8]> = Vec::new();

    // Add spl_mint hash
    hash_inputs.push(hashed_spl_mint.as_slice());

    // Add supply bytes
    hash_inputs.push(supply_bytes.as_slice());

    // Add decimals with prefix if not 0 (our decimals = 8, so add it)
    let mut decimals_bytes = [0u8; 32];
    if decimals != 0 {
        decimals_bytes[30] = 1; // decimals prefix
        decimals_bytes[31] = decimals;
        hash_inputs.push(&decimals_bytes[..]);
    }

    // is_decompressed = false, so skip

    // Add mint authority if present
    hash_inputs.push(hashed_mint_authority.as_slice());

    // Add freeze authority if present
    hash_inputs.push(hashed_freeze_authority.as_slice());

    // version = 0, so skip

    let base_mint_hash = Poseidon::hashv(&hash_inputs).unwrap();

    // 5. Calculate extension hash manually
    let extension_hashchain = token_metadata.hash().unwrap();

    // 7. Combine base mint hash with extension hashchain
    let manual_hash =
        Poseidon::hashv(&[base_mint_hash.as_slice(), extension_hashchain.as_slice()]).unwrap();

    // Verify that calculated hash matches manual hash
    assert_eq!(
        calculated_hash, manual_hash,
        "CompressedMint::hash() with extension should match manual hash calculation"
    );
}

#[track_caller]
fn assert_to_previous_hashes(hash: [u8; 32], previous_hashes: &mut Vec<[u8; 32]>) {
    for previous_hash in previous_hashes.iter() {
        assert_ne!(hash, *previous_hash, "Hash collision detected!");
    }
    previous_hashes.push(hash);
}

#[test]
fn test_compressed_mint_hash_collision_detection() {
    let mut previous_hashes = Vec::new();

    // Base configuration - choose default state for each field
    let base = CompressedMint {
        version: 0,                                    // Base: version 0 (Poseidon)
        spl_mint: Pubkey::new_from_array([100u8; 32]), // Base: specific pubkey
        supply: 1000000,                               // Base: 1M tokens
        decimals: 6,                                   // Base: 6 decimals
        is_decompressed: false,                        // Base: false
        mint_authority: None,                          // Base: None
        freeze_authority: None,                        // Base: None
        extensions: None,                              // Base: None
    };

    assert_to_previous_hashes(base.hash().unwrap(), &mut previous_hashes);

    // Test ALL version states (Rule 2: All States Must Be Tested)
    for version in [1] {
        // Test version 1 (SHA256BE), skip 0 as it's base
        let mut variant = base.clone();
        variant.version = version;
        assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);
    }

    // Test different spl_mint values
    for mint_bytes in [[1u8; 32], [200u8; 32], [255u8; 32]] {
        let mut variant = base.clone();
        variant.spl_mint = Pubkey::new_from_array(mint_bytes);
        assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);
    }

    // Test different supply values (numeric field - boundary values)
    for supply in [0, 1, 42, u64::MAX] {
        let mut variant = base.clone();
        variant.supply = supply;
        assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);
    }

    // Test different decimals values
    for decimals in [0, 1, 9, 18, 255] {
        if decimals == base.decimals {
            continue;
        } // Skip base state
        let mut variant = base.clone();
        variant.decimals = decimals;
        assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);
    }

    // Test ALL boolean states for is_decompressed
    let mut variant = base.clone();
    variant.is_decompressed = !base.is_decompressed; // Test opposite state
    assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

    // Test ALL states for mint_authority Option field
    let mut variant = base.clone();
    variant.mint_authority = Some(Pubkey::new_from_array([50u8; 32])); // Test Some state
    assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

    // Test ALL states for freeze_authority Option field
    let mut variant = base.clone();
    variant.freeze_authority = Some(Pubkey::new_from_array([75u8; 32])); // Test Some state
    assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

    // Test ALL states for extensions Option field
    // Create different extension states
    let token_metadata = TokenMetadata {
        update_authority: Some(Pubkey::new_from_array([10u8; 32])),
        mint: base.spl_mint,
        metadata: Metadata {
            name: b"Test".to_vec(),
            symbol: b"TST".to_vec(),
            uri: b"https://test.com".to_vec(),
        },
        additional_metadata: vec![],
        version: 0,
    };

    // Test Some state with single extension
    let mut variant = base.clone();
    variant.extensions = Some(vec![ExtensionStruct::TokenMetadata(token_metadata.clone())]);
    assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

    // Test Some state with multiple extensions (if more extension types existed)
    // Note: Only TokenMetadata is available, so we'll create variations
    let mut different_metadata = token_metadata.clone();
    different_metadata.metadata.name = b"Different".to_vec();

    let mut variant = base.clone();
    variant.extensions = Some(vec![
        ExtensionStruct::TokenMetadata(token_metadata.clone()),
        ExtensionStruct::TokenMetadata(different_metadata),
    ]);
    assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

    // Critical Security Test: Authority confusion prevention
    // Test same pubkey in different authority positions - ALL combinations
    let same_pubkey = Pubkey::new_from_array([42u8; 32]);

    let authority_combinations = [
        (None, None),                           // Base case
        (Some(same_pubkey), None),              // Only mint authority
        (None, Some(same_pubkey)),              // Only freeze authority
        (Some(same_pubkey), Some(same_pubkey)), // Both authorities same
    ];

    for (mint_auth, freeze_auth) in authority_combinations {
        if mint_auth == base.mint_authority && freeze_auth == base.freeze_authority {
            continue; // Skip base case
        }
        let mut variant = base.clone();
        variant.mint_authority = mint_auth;
        variant.freeze_authority = freeze_auth;
        assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);
    }

    // Test with different pubkeys in authorities
    let different_pubkey1 = Pubkey::new_from_array([111u8; 32]);
    let different_pubkey2 = Pubkey::new_from_array([222u8; 32]);

    let mut variant = base.clone();
    variant.mint_authority = Some(different_pubkey1);
    variant.freeze_authority = Some(different_pubkey2);
    assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);
}

#[test]
fn test_compressed_mint_zero_value_edge_cases() {
    let mut previous_hashes = Vec::new();

    // All fields zero/minimal/None values - the "zero state"
    let all_zero = CompressedMint {
        version: 0,                                  // Zero version
        spl_mint: Pubkey::new_from_array([0u8; 32]), // Zero pubkey
        supply: 0,                                   // Zero supply
        decimals: 0,                                 // Zero decimals
        is_decompressed: false,                      // False (zero-like)
        mint_authority: None,                        // None (zero-like)
        freeze_authority: None,                      // None (zero-like)
        extensions: None,                            // None (zero-like)
    };

    assert_to_previous_hashes(all_zero.hash().unwrap(), &mut previous_hashes);

    // Test one field non-zero, all others zero
    // This tests if changing only one field from zero state produces unique hash

    // Only version non-zero
    let mut variant = all_zero.clone();
    variant.version = 1;
    assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

    // Only spl_mint non-zero
    let mut variant = all_zero.clone();
    variant.spl_mint = Pubkey::new_from_array([1u8; 32]);
    assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

    // Only supply non-zero
    let mut variant = all_zero.clone();
    variant.supply = 1;
    assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

    // Only decimals non-zero
    let mut variant = all_zero.clone();
    variant.decimals = 1;
    assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

    // Only is_decompressed non-zero (true)
    let mut variant = all_zero.clone();
    variant.is_decompressed = true;
    assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

    // Only mint_authority non-zero
    let mut variant = all_zero.clone();
    variant.mint_authority = Some(Pubkey::new_from_array([1u8; 32]));
    assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

    // Only freeze_authority non-zero
    let mut variant = all_zero.clone();
    variant.freeze_authority = Some(Pubkey::new_from_array([1u8; 32]));
    assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

    // Only extensions non-zero (minimal extension)
    let minimal_metadata = TokenMetadata {
        update_authority: None,
        mint: Pubkey::new_from_array([0u8; 32]), // Use zero mint to keep other fields zero
        metadata: Metadata {
            name: vec![1u8], // Minimal non-empty
            symbol: vec![],  // Keep empty
            uri: vec![],     // Keep empty
        },
        additional_metadata: vec![],
        version: 0,
    };

    let mut variant = all_zero.clone();
    variant.extensions = Some(vec![ExtensionStruct::TokenMetadata(minimal_metadata)]);
    assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

    // Critical edge case: Test minimal increments from zero
    // This ensures the hash function has good sensitivity at low values

    // Test supply: 0 vs 1 vs 2 (ensuring small value changes are detected)
    for supply in [2, 3] {
        let mut variant = all_zero.clone();
        variant.supply = supply;
        assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);
    }

    // Test decimals: 0 vs 1 vs 2
    for decimals in [2, 3] {
        let mut variant = all_zero.clone();
        variant.decimals = decimals;
        assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);
    }

    // Test pubkey sensitivity: all zeros vs single bit flip
    let single_bit_pubkey = {
        let mut bytes = [0u8; 32];
        bytes[31] = 1; // Flip only the last bit
        Pubkey::new_from_array(bytes)
    };

    let mut variant = all_zero.clone();
    variant.spl_mint = single_bit_pubkey;
    assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

    // Test authority sensitivity: None vs zero pubkey vs non-zero pubkey
    let zero_pubkey = Pubkey::new_from_array([0u8; 32]);
    let mut variant = all_zero.clone();
    variant.mint_authority = Some(zero_pubkey);
    assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);
}
