//! AMM ObservationState Tests: ObservationState trait derive tests
//!
//! Tests each trait derived by `LightAccount` macro for `ObservationState`:
//! - LightHasherSha -> DataHasher + ToByteArray
//! - LightDiscriminator -> LIGHT_DISCRIMINATOR constant
//! - Compressible -> HasCompressionInfo + CompressAs + Size + CompressedInitSpace
//! - CompressiblePack -> Pack + Unpack + PackedObservationState
//!
//! ObservationState has 1 Pubkey field (pool_id) and a nested array of Observation structs,
//! testing Pack/Unpack behavior with array fields and nested data structures.

use csdk_anchor_full_derived_test::{Observation, ObservationState, PackedObservationState};
use light_hasher::{DataHasher, Sha256};
use light_sdk::{
    compressible::{CompressAs, CompressionInfo, Pack},
    instruction::PackedAccounts,
};
use solana_pubkey::Pubkey;

use super::shared::CompressibleTestFactory;
use crate::generate_trait_tests;

// =============================================================================
// Factory Implementation
// =============================================================================

impl CompressibleTestFactory for ObservationState {
    fn with_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::default(),
            initialized: false,
            observation_index: 0,
            pool_id: Pubkey::new_unique(),
            observations: [
                Observation {
                    block_timestamp: 0,
                    cumulative_token_0_price_x32: 0,
                    cumulative_token_1_price_x32: 0,
                },
                Observation {
                    block_timestamp: 0,
                    cumulative_token_0_price_x32: 0,
                    cumulative_token_1_price_x32: 0,
                },
            ],
            padding: [0u64; 4],
        }
    }

    fn without_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::compressed(),
            initialized: false,
            observation_index: 0,
            pool_id: Pubkey::new_unique(),
            observations: [
                Observation {
                    block_timestamp: 0,
                    cumulative_token_0_price_x32: 0,
                    cumulative_token_1_price_x32: 0,
                },
                Observation {
                    block_timestamp: 0,
                    cumulative_token_0_price_x32: 0,
                    cumulative_token_1_price_x32: 0,
                },
            ],
            padding: [0u64; 4],
        }
    }
}

// =============================================================================
// Generate all generic trait tests via macro
// =============================================================================

generate_trait_tests!(ObservationState);

// =============================================================================
// Struct-Specific CompressAs Tests
// =============================================================================

#[test]
fn test_compress_as_preserves_pool_id() {
    let pool_id = Pubkey::new_unique();

    let observation_state = ObservationState {
        compression_info: CompressionInfo::default(),
        initialized: true,
        observation_index: 5,
        pool_id,
        observations: [
            Observation {
                block_timestamp: 1000,
                cumulative_token_0_price_x32: 100,
                cumulative_token_1_price_x32: 200,
            },
            Observation {
                block_timestamp: 2000,
                cumulative_token_0_price_x32: 300,
                cumulative_token_1_price_x32: 400,
            },
        ],
        padding: [0u64; 4],
    };

    let compressed = observation_state.compress_as();
    let inner = compressed.into_owned();

    assert_eq!(inner.pool_id, pool_id);
    assert!(inner.initialized);
    assert_eq!(inner.observation_index, 5);
}

#[test]
fn test_compress_as_preserves_observation_data() {
    let observation_state = ObservationState {
        compression_info: CompressionInfo::default(),
        initialized: true,
        observation_index: 1,
        pool_id: Pubkey::new_unique(),
        observations: [
            Observation {
                block_timestamp: 1111,
                cumulative_token_0_price_x32: 5000,
                cumulative_token_1_price_x32: 6000,
            },
            Observation {
                block_timestamp: 2222,
                cumulative_token_0_price_x32: 7000,
                cumulative_token_1_price_x32: 8000,
            },
        ],
        padding: [10, 20, 30, 40],
    };

    let compressed = observation_state.compress_as();
    let inner = compressed.into_owned();

    assert_eq!(inner.observations[0].block_timestamp, 1111);
    assert_eq!(inner.observations[0].cumulative_token_0_price_x32, 5000);
    assert_eq!(inner.observations[0].cumulative_token_1_price_x32, 6000);
    assert_eq!(inner.observations[1].block_timestamp, 2222);
    assert_eq!(inner.observations[1].cumulative_token_0_price_x32, 7000);
    assert_eq!(inner.observations[1].cumulative_token_1_price_x32, 8000);
    assert_eq!(inner.padding, [10, 20, 30, 40]);
}

// =============================================================================
// Struct-Specific DataHasher Tests
// =============================================================================

#[test]
fn test_hash_differs_for_different_pool_id() {
    let mut observation1 = ObservationState::without_compression_info();
    let mut observation2 = ObservationState::without_compression_info();

    observation1.pool_id = Pubkey::new_unique();
    observation2.pool_id = Pubkey::new_unique();

    let hash1 = observation1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = observation2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different pool_id should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_initialized() {
    let mut observation1 = ObservationState::without_compression_info();
    let mut observation2 = ObservationState::without_compression_info();

    observation1.initialized = true;
    observation2.initialized = false;

    let hash1 = observation1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = observation2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different initialized should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_observation_index() {
    let mut observation1 = ObservationState::without_compression_info();
    let mut observation2 = ObservationState::without_compression_info();

    observation1.observation_index = 1;
    observation2.observation_index = 2;

    let hash1 = observation1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = observation2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different observation_index should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_observation_data() {
    let mut observation1 = ObservationState::without_compression_info();
    let mut observation2 = ObservationState::without_compression_info();

    observation1.observations[0].block_timestamp = 1000;
    observation2.observations[0].block_timestamp = 2000;

    let hash1 = observation1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = observation2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different observation data should produce different hash"
    );
}

// =============================================================================
// Pack/Unpack Tests (struct-specific, cannot be generic)
// =============================================================================

#[test]
fn test_packed_struct_has_u8_pool_id_index() {
    // ObservationState has 1 Pubkey field (pool_id), so PackedObservationState should have 1 u8 field
    let packed = PackedObservationState {
        initialized: false,
        observation_index: 0,
        pool_id: 0,
        observations: [
            Observation {
                block_timestamp: 0,
                cumulative_token_0_price_x32: 0,
                cumulative_token_1_price_x32: 0,
            },
            Observation {
                block_timestamp: 0,
                cumulative_token_0_price_x32: 0,
                cumulative_token_1_price_x32: 0,
            },
        ],
        padding: [0u64; 4],
    };

    assert_eq!(packed.pool_id, 0u8);
}

#[test]
fn test_pack_converts_pool_id_to_index() {
    let pool_id = Pubkey::new_unique();

    let observation_state = ObservationState {
        compression_info: CompressionInfo::compressed(),
        initialized: true,
        observation_index: 0,
        pool_id,
        observations: [
            Observation {
                block_timestamp: 0,
                cumulative_token_0_price_x32: 0,
                cumulative_token_1_price_x32: 0,
            },
            Observation {
                block_timestamp: 0,
                cumulative_token_0_price_x32: 0,
                cumulative_token_1_price_x32: 0,
            },
        ],
        padding: [0u64; 4],
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = observation_state.pack(&mut packed_accounts).unwrap();

    // The pool_id should have been added to packed_accounts and assigned index 0
    assert_eq!(packed.pool_id, 0u8);

    let stored_pubkeys = packed_accounts.packed_pubkeys();
    assert_eq!(stored_pubkeys.len(), 1);
    assert_eq!(stored_pubkeys[0], pool_id.to_bytes());
}

#[test]
fn test_pack_with_pre_existing_pubkeys() {
    let pool_id = Pubkey::new_unique();

    let observation_state = ObservationState {
        compression_info: CompressionInfo::compressed(),
        initialized: false,
        observation_index: 0,
        pool_id,
        observations: [
            Observation {
                block_timestamp: 0,
                cumulative_token_0_price_x32: 0,
                cumulative_token_1_price_x32: 0,
            },
            Observation {
                block_timestamp: 0,
                cumulative_token_0_price_x32: 0,
                cumulative_token_1_price_x32: 0,
            },
        ],
        padding: [0u64; 4],
    };

    let mut packed_accounts = PackedAccounts::default();
    // Pre-insert another pubkey
    packed_accounts.insert_or_get(Pubkey::new_unique());

    let packed = observation_state.pack(&mut packed_accounts).unwrap();

    // The pool_id should have been added and assigned index 1 (since index 0 is taken)
    assert_eq!(packed.pool_id, 1u8);
}

#[test]
fn test_pack_preserves_all_fields() {
    let pool_id = Pubkey::new_unique();

    let observation_state = ObservationState {
        compression_info: CompressionInfo::compressed(),
        initialized: true,
        observation_index: 42,
        pool_id,
        observations: [
            Observation {
                block_timestamp: 1000,
                cumulative_token_0_price_x32: 5000,
                cumulative_token_1_price_x32: 6000,
            },
            Observation {
                block_timestamp: 2000,
                cumulative_token_0_price_x32: 7000,
                cumulative_token_1_price_x32: 8000,
            },
        ],
        padding: [111, 222, 333, 444],
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = observation_state.pack(&mut packed_accounts).unwrap();

    assert!(packed.initialized);
    assert_eq!(packed.observation_index, 42);
    assert_eq!(packed.observations[0].block_timestamp, 1000);
    assert_eq!(packed.observations[0].cumulative_token_0_price_x32, 5000);
    assert_eq!(packed.observations[0].cumulative_token_1_price_x32, 6000);
    assert_eq!(packed.observations[1].block_timestamp, 2000);
    assert_eq!(packed.observations[1].cumulative_token_0_price_x32, 7000);
    assert_eq!(packed.observations[1].cumulative_token_1_price_x32, 8000);
    assert_eq!(packed.padding, [111, 222, 333, 444]);
}

#[test]
fn test_pack_different_pool_ids_get_different_indices() {
    let pool_id1 = Pubkey::new_unique();
    let pool_id2 = Pubkey::new_unique();

    let observation1 = ObservationState {
        compression_info: CompressionInfo::compressed(),
        initialized: false,
        observation_index: 0,
        pool_id: pool_id1,
        observations: [
            Observation {
                block_timestamp: 0,
                cumulative_token_0_price_x32: 0,
                cumulative_token_1_price_x32: 0,
            },
            Observation {
                block_timestamp: 0,
                cumulative_token_0_price_x32: 0,
                cumulative_token_1_price_x32: 0,
            },
        ],
        padding: [0u64; 4],
    };

    let observation2 = ObservationState {
        compression_info: CompressionInfo::compressed(),
        initialized: false,
        observation_index: 0,
        pool_id: pool_id2,
        observations: [
            Observation {
                block_timestamp: 0,
                cumulative_token_0_price_x32: 0,
                cumulative_token_1_price_x32: 0,
            },
            Observation {
                block_timestamp: 0,
                cumulative_token_0_price_x32: 0,
                cumulative_token_1_price_x32: 0,
            },
        ],
        padding: [0u64; 4],
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = observation1.pack(&mut packed_accounts).unwrap();
    let packed2 = observation2.pack(&mut packed_accounts).unwrap();

    // Different pool IDs should get different indices
    assert_ne!(
        packed1.pool_id, packed2.pool_id,
        "different pool_ids should produce different indices"
    );
}

#[test]
fn test_pack_reuses_same_pool_id_index() {
    let pool_id = Pubkey::new_unique();

    let observation1 = ObservationState {
        compression_info: CompressionInfo::compressed(),
        initialized: false,
        observation_index: 0,
        pool_id,
        observations: [
            Observation {
                block_timestamp: 1000,
                cumulative_token_0_price_x32: 100,
                cumulative_token_1_price_x32: 200,
            },
            Observation {
                block_timestamp: 0,
                cumulative_token_0_price_x32: 0,
                cumulative_token_1_price_x32: 0,
            },
        ],
        padding: [0u64; 4],
    };

    let observation2 = ObservationState {
        compression_info: CompressionInfo::compressed(),
        initialized: true,
        observation_index: 1,
        pool_id,
        observations: [
            Observation {
                block_timestamp: 2000,
                cumulative_token_0_price_x32: 300,
                cumulative_token_1_price_x32: 400,
            },
            Observation {
                block_timestamp: 0,
                cumulative_token_0_price_x32: 0,
                cumulative_token_1_price_x32: 0,
            },
        ],
        padding: [0u64; 4],
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = observation1.pack(&mut packed_accounts).unwrap();
    let packed2 = observation2.pack(&mut packed_accounts).unwrap();

    // Same pool_id should get same index
    assert_eq!(
        packed1.pool_id, packed2.pool_id,
        "same pool_id should produce same index"
    );
}

#[test]
fn test_pack_stores_pool_id_in_packed_accounts() {
    let pool_id = Pubkey::new_unique();

    let observation_state = ObservationState {
        compression_info: CompressionInfo::compressed(),
        initialized: false,
        observation_index: 0,
        pool_id,
        observations: [
            Observation {
                block_timestamp: 0,
                cumulative_token_0_price_x32: 0,
                cumulative_token_1_price_x32: 0,
            },
            Observation {
                block_timestamp: 0,
                cumulative_token_0_price_x32: 0,
                cumulative_token_1_price_x32: 0,
            },
        ],
        padding: [0u64; 4],
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = observation_state.pack(&mut packed_accounts).unwrap();

    let stored_pubkeys = packed_accounts.packed_pubkeys();
    assert_eq!(stored_pubkeys.len(), 1, "should have 1 pubkey stored");
    assert_eq!(
        stored_pubkeys[packed.pool_id as usize], pool_id.to_bytes(),
        "stored pubkey should match"
    );
}
