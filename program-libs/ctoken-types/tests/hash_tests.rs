// #[cfg(test)]
// mod hash_tests {
//     use light_compressed_account::Pubkey;
//     use light_ctoken_types::state::{BaseMint, CompressedMint, CompressedMintMetadata};
//     use rand::Rng;

//     /// Hash Collision Detection Tests
//     /// Tests for CompressedMint::hash() following hash_collision_testing_guide.md:
//     ///
//     /// 1. test_hash_basic_functionality - Basic functionality and determinism
//     /// 2. test_hash_collision_detection - Systematic field-by-field collision testing
//     /// 3. test_hash_zero_value_edge_cases - Edge cases with zero/minimal values
//     /// 4. test_hash_boundary_values - Boundary value testing for numeric fields
//     /// 5. test_hash_authority_combinations - Authority confusion prevention
//     /// 6. test_hash_randomized_1k_iterations - Randomized testing with 1k iterations
//     /// 7. test_hash_some_zero_vs_none - Some(zero) vs None semantic distinction
//     ///
//     ///    Helper function for collision detection - reuse existing pattern from token_data.rs
//     fn assert_to_previous_hashes(hash: [u8; 32], previous_hashes: &mut Vec<[u8; 32]>) {
//         for previous_hash in previous_hashes.iter() {
//             assert_ne!(hash, *previous_hash, "Hash collision detected!");
//         }
//         previous_hashes.push(hash);
//     }

//     #[test]
//     fn test_hash_basic_functionality() {
//         let mint = CompressedMint {
//             base: BaseMint {
//                 mint_authority: Some(Pubkey::new_unique()),
//                 supply: 1000000,
//                 decimals: 6,
//                 is_initialized: true,
//                 freeze_authority: Some(Pubkey::new_unique()),
//             },
//             metadata: CompressedMintMetadata {
//                 version: 3,
//                 mint: Pubkey::new_unique(),
//                 spl_mint_initialized: false,
//             },
//             extensions: None,
//         };

//         // Test basic functionality
//         let hash_result = mint.hash().unwrap();
//         assert_eq!(hash_result.len(), 32);
//         assert_ne!(hash_result, [0u8; 32]); // Not empty hash

//         // Test determinism - same input produces same hash
//         let hash_result2 = mint.hash().unwrap();
//         assert_eq!(hash_result, hash_result2);

//         // Test version validation - only version 3 supported
//         let mut invalid_mint = mint.clone();
//         invalid_mint.metadata.version = 0;
//         assert!(invalid_mint.hash().is_err());

//         invalid_mint.metadata.version = 1;
//         assert!(invalid_mint.hash().is_err());

//         invalid_mint.metadata.version = 2;
//         assert!(invalid_mint.hash().is_err());

//         invalid_mint.metadata.version = 4;
//         assert!(invalid_mint.hash().is_err());
//     }

//     #[test]
//     fn test_hash_collision_detection() {
//         let mut previous_hashes = Vec::new();

//         // Base configuration - choose default state for each field
//         let base = CompressedMint {
//             base: BaseMint {
//                 mint_authority: None,
//                 supply: 0,
//                 decimals: 0,
//                 is_initialized: true,
//                 freeze_authority: None,
//             },
//             metadata: CompressedMintMetadata {
//                 version: 3,
//                 mint: Pubkey::new_from_array([1u8; 32]),
//                 spl_mint_initialized: false,
//             },
//             extensions: None,
//         };

//         assert_to_previous_hashes(base.hash().unwrap(), &mut previous_hashes);

//         // Test different mint values
//         for i in 2u8..10u8 {
//             let mut variant = base.clone();
//             variant.metadata.mint = Pubkey::new_from_array([i; 32]);
//             assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);
//         }

//         // Test different supply values
//         for supply in [1, 42, 1000, u64::MAX] {
//             let mut variant = base.clone();
//             variant.base.supply = supply;
//             assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);
//         }

//         // Test different decimals values
//         for decimals in [1, 6, 9, 18, u8::MAX] {
//             let mut variant = base.clone();
//             variant.base.decimals = decimals;
//             assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);
//         }

//         // Test spl_mint_initialized boolean states
//         let mut variant = base.clone();
//         variant.metadata.spl_mint_initialized = true; // Flip from false
//         assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

//         // Test mint_authority Option states
//         let mut variant = base.clone();
//         variant.base.mint_authority = Some(Pubkey::new_from_array([10u8; 32]));
//         assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

//         // Test freeze_authority Option states
//         let mut variant = base.clone();
//         variant.base.freeze_authority = Some(Pubkey::new_from_array([11u8; 32]));
//         assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

//         // Test extensions Option states
//         let mut variant = base.clone();
//         variant.extensions = Some(vec![]); // Empty vec vs None
//         assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

//         // Test multiple fields changed simultaneously
//         let mut variant = base.clone();
//         variant.base.supply = 5000;
//         variant.base.decimals = 9;
//         variant.metadata.spl_mint_initialized = true;
//         variant.base.mint_authority = Some(Pubkey::new_from_array([12u8; 32]));
//         variant.base.freeze_authority = Some(Pubkey::new_from_array([13u8; 32]));
//         variant.extensions = Some(vec![]);
//         assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);
//     }

//     #[test]
//     fn test_hash_zero_value_edge_cases() {
//         let mut previous_hashes = Vec::new();

//         // All fields zero/None/false (minimal state)
//         let all_minimal = CompressedMint {
//             base: BaseMint {
//                 mint_authority: None,
//                 supply: 0,
//                 decimals: 0,
//                 is_initialized: true,
//                 freeze_authority: None,
//             },
//             metadata: CompressedMintMetadata {
//                 version: 3,
//                 mint: Pubkey::new_from_array([0u8; 32]),
//                 spl_mint_initialized: false,
//             },
//             extensions: None,
//         };
//         assert_to_previous_hashes(all_minimal.hash().unwrap(), &mut previous_hashes);

//         // Test each field individually set to non-zero while others remain minimal
//         let mut variant = all_minimal.clone();
//         variant.metadata.mint = Pubkey::new_from_array([1u8; 32]); // Only this field non-zero
//         assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

//         variant = all_minimal.clone();
//         variant.base.supply = 1; // Only this field non-zero
//         assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

//         variant = all_minimal.clone();
//         variant.base.decimals = 1; // Only this field non-zero
//         assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

//         variant = all_minimal.clone();
//         variant.metadata.spl_mint_initialized = true; // Only this field non-false
//         assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

//         variant = all_minimal.clone();
//         variant.base.mint_authority = Some(Pubkey::new_from_array([1u8; 32])); // Only this field non-None
//         assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

//         variant = all_minimal.clone();
//         variant.base.freeze_authority = Some(Pubkey::new_from_array([2u8; 32])); // Only this field non-None
//         assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);

//         variant = all_minimal.clone();
//         variant.extensions = Some(vec![]); // Only this field non-None
//         assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);
//     }

//     #[test]
//     fn test_hash_boundary_values() {
//         let mut previous_hashes = Vec::new();

//         let base = CompressedMint {
//             base: BaseMint {
//                 mint_authority: Some(Pubkey::new_from_array([2u8; 32])),
//                 supply: 100,
//                 decimals: 6,
//                 is_initialized: true,
//                 freeze_authority: Some(Pubkey::new_from_array([3u8; 32])),
//             },
//             metadata: CompressedMintMetadata {
//                 version: 3,
//                 mint: Pubkey::new_from_array([1u8; 32]),
//                 spl_mint_initialized: false,
//             },
//             extensions: None,
//         };
//         assert_to_previous_hashes(base.hash().unwrap(), &mut previous_hashes);

//         // Test supply boundaries - avoid duplicating base value 100
//         for supply in [0, 1, 2, u32::MAX as u64, u64::MAX - 1, u64::MAX] {
//             if supply == 100 {
//                 continue;
//             } // Skip base value
//             let mut variant = base.clone();
//             variant.base.supply = supply;
//             assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);
//         }

//         // Test decimals boundaries - avoid duplicating base value 6
//         for decimals in [0, 1, 2, 9, 18, u8::MAX - 1, u8::MAX] {
//             if decimals == 6 {
//                 continue;
//             } // Skip base value
//             let mut variant = base.clone();
//             variant.base.decimals = decimals;
//             assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);
//         }

//         // Test pubkey boundaries (edge bytes in array) - avoid duplicating base values
//         for pubkey_bytes in [[0u8; 32], [4u8; 32], [255u8; 32]] {
//             let mut variant = base.clone();
//             variant.metadata.mint = Pubkey::new_from_array(pubkey_bytes);
//             assert_to_previous_hashes(variant.hash().unwrap(), &mut previous_hashes);
//         }
//     }

//     #[test]
//     fn test_hash_authority_combinations() {
//         let mut previous_hashes = Vec::new();
//         let same_pubkey = Pubkey::new_from_array([42u8; 32]);

//         let base = CompressedMint {
//             base: BaseMint {
//                 mint_authority: None,
//                 supply: 1000,
//                 decimals: 6,
//                 is_initialized: true,
//                 freeze_authority: None,
//             },
//             metadata: CompressedMintMetadata {
//                 version: 3,
//                 mint: Pubkey::new_from_array([1u8; 32]),
//                 spl_mint_initialized: false,
//             },
//             extensions: None,
//         };

//         // Test all authority combinations with same pubkey - must produce different hashes

//         // Case 1: None mint_authority, None freeze_authority
//         let mut variant1 = base.clone();
//         variant1.base.mint_authority = None;
//         variant1.base.freeze_authority = None;
//         let hash1 = variant1.hash().unwrap();
//         assert_to_previous_hashes(hash1, &mut previous_hashes);

//         // Case 2: Some mint_authority, None freeze_authority (using same pubkey)
//         let mut variant2 = base.clone();
//         variant2.base.mint_authority = Some(same_pubkey);
//         variant2.base.freeze_authority = None;
//         let hash2 = variant2.hash().unwrap();
//         assert_to_previous_hashes(hash2, &mut previous_hashes);

//         // Case 3: None mint_authority, Some freeze_authority (using same pubkey)
//         let mut variant3 = base.clone();
//         variant3.base.mint_authority = None;
//         variant3.base.freeze_authority = Some(same_pubkey);
//         let hash3 = variant3.hash().unwrap();
//         assert_to_previous_hashes(hash3, &mut previous_hashes);

//         // Case 4: Both authorities present (using same pubkey)
//         let mut variant4 = base.clone();
//         variant4.base.mint_authority = Some(same_pubkey);
//         variant4.base.freeze_authority = Some(same_pubkey);
//         let hash4 = variant4.hash().unwrap();
//         assert_to_previous_hashes(hash4, &mut previous_hashes);

//         // Critical security check: all combinations must produce different hashes
//         assert_ne!(
//             hash1, hash2,
//             "CRITICAL: Hash collision between different authority configurations!"
//         );
//         assert_ne!(
//             hash1, hash3,
//             "CRITICAL: Hash collision between different authority configurations!"
//         );
//         assert_ne!(
//             hash1, hash4,
//             "CRITICAL: Hash collision between different authority configurations!"
//         );
//         assert_ne!(
//             hash2, hash3,
//             "CRITICAL: Hash collision between different authority configurations!"
//         );
//         assert_ne!(
//             hash2, hash4,
//             "CRITICAL: Hash collision between different authority configurations!"
//         );
//         assert_ne!(
//             hash3, hash4,
//             "CRITICAL: Hash collision between different authority configurations!"
//         );
//     }

//     #[test]
//     fn test_hash_some_zero_vs_none() {
//         let pubkey_zero = Pubkey::new_from_array([0u8; 32]);

//         let base = CompressedMint {
//             base: BaseMint {
//                 mint_authority: None,
//                 supply: 1000,
//                 decimals: 6,
//                 is_initialized: true,
//                 freeze_authority: None,
//             },
//             metadata: CompressedMintMetadata {
//                 version: 3,
//                 mint: Pubkey::new_from_array([1u8; 32]),
//                 spl_mint_initialized: false,
//             },
//             extensions: None,
//         };

//         // Test Some(zero_pubkey) vs None for mint_authority
//         let mut variant_none = base.clone();
//         variant_none.base.mint_authority = None;
//         let hash_none = variant_none.hash().unwrap();

//         let mut variant_some_zero = base.clone();
//         variant_some_zero.base.mint_authority = Some(pubkey_zero);
//         let hash_some_zero = variant_some_zero.hash().unwrap();

//         assert_ne!(
//             hash_none, hash_some_zero,
//             "Some(zero_pubkey) must hash differently from None for mint_authority!"
//         );

//         // Test Some(zero_pubkey) vs None for freeze_authority
//         let mut variant_none_freeze = base.clone();
//         variant_none_freeze.base.freeze_authority = None;
//         let hash_none_freeze = variant_none_freeze.hash().unwrap();

//         let mut variant_some_zero_freeze = base.clone();
//         variant_some_zero_freeze.base.freeze_authority = Some(pubkey_zero);
//         let hash_some_zero_freeze = variant_some_zero_freeze.hash().unwrap();

//         assert_ne!(
//             hash_none_freeze, hash_some_zero_freeze,
//             "Some(zero_pubkey) must hash differently from None for freeze_authority!"
//         );

//         // Test Some(empty_vec) vs None for extensions
//         let mut variant_none_ext = base.clone();
//         variant_none_ext.extensions = None;
//         let hash_none_ext = variant_none_ext.hash().unwrap();

//         let mut variant_some_empty_ext = base.clone();
//         variant_some_empty_ext.extensions = Some(vec![]);
//         let hash_some_empty_ext = variant_some_empty_ext.hash().unwrap();

//         assert_ne!(
//             hash_none_ext, hash_some_empty_ext,
//             "Some(empty_vec) must hash differently from None for extensions!"
//         );
//     }

//     #[test]
//     fn test_hash_randomized_1k_iterations() {
//         // Use thread RNG following existing test patterns
//         let mut rng = rand::thread_rng();
//         let mut all_hashes = Vec::new();

//         for iteration in 0..1000 {
//             let mint = CompressedMint {
//                 base: BaseMint {
//                     mint_authority: if rng.gen_bool(0.7) {
//                         Some(Pubkey::new_from_array(rng.gen::<[u8; 32]>()))
//                     } else {
//                         None
//                     },
//                     supply: rng.gen::<u64>(),
//                     decimals: rng.gen_range(0..=18), // Realistic decimal range
//                     is_initialized: true,
//                     freeze_authority: if rng.gen_bool(0.7) {
//                         Some(Pubkey::new_from_array(rng.gen::<[u8; 32]>()))
//                     } else {
//                         None
//                     },
//                 },
//                 metadata: CompressedMintMetadata {
//                     version: 3, // Always version 3
//                     mint: Pubkey::new_from_array(rng.gen::<[u8; 32]>()),
//                     spl_mint_initialized: rng.gen_bool(0.5),
//                 },
//                 extensions: if rng.gen_bool(0.3) {
//                     Some(vec![]) // Empty extensions for now
//                 } else {
//                     None
//                 },
//             };

//             let hash_result = mint.hash().unwrap();

//             // Basic validation
//             assert_eq!(hash_result.len(), 32);
//             assert_ne!(hash_result, [0u8; 32]); // Should not be all zeros

//             // Test determinism - same mint should produce same hash
//             let hash_result2 = mint.hash().unwrap();
//             assert_eq!(
//                 hash_result, hash_result2,
//                 "Hash function is not deterministic at iteration {}",
//                 iteration
//             );

//             // Check for collisions with all previous hashes
//             for (prev_iteration, prev_hash) in all_hashes.iter().enumerate() {
//                 assert_ne!(
//                     hash_result, *prev_hash,
//                     "Hash collision detected! Iteration {} collides with iteration {}",
//                     iteration, prev_iteration
//                 );
//             }

//             all_hashes.push(hash_result);
//         }

//         println!(
//             "Successfully tested {} random mint configurations without collisions",
//             all_hashes.len()
//         );
//     }
// }
