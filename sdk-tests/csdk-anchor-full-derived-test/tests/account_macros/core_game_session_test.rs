//! Core Tests: GameSession trait derive tests
//!
//! Tests each trait derived by `LightAccount` macro for `GameSession`:
//! - LightHasherSha -> DataHasher + ToByteArray
//! - LightDiscriminator -> LIGHT_DISCRIMINATOR constant
//! - Compressible -> HasCompressionInfo + CompressAs + Size + CompressedInitSpace
//! - CompressiblePack -> Pack + Unpack + PackedGameSession
//!
//! GameSession has #[compress_as(start_time = 0, end_time = None, score = 0)]
//! which overrides field values during compression.

use csdk_anchor_full_derived_test::{GameSession, PackedGameSession};
use light_hasher::{DataHasher, Sha256};
use light_account::{CompressAs, CompressionInfo, Pack};
use light_sdk::instruction::PackedAccounts;
use solana_pubkey::Pubkey;

use super::shared::CompressibleTestFactory;
use crate::generate_trait_tests;

// =============================================================================
// Factory Implementation
// =============================================================================

impl CompressibleTestFactory for GameSession {
    fn with_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::default(),
            session_id: 1,
            player: Pubkey::new_unique(),
            game_type: "test game".to_string(),
            start_time: 100,
            end_time: Some(200),
            score: 50,
        }
    }

    fn without_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::compressed(),
            session_id: 1,
            player: Pubkey::new_unique(),
            game_type: "test game".to_string(),
            start_time: 100,
            end_time: Some(200),
            score: 50,
        }
    }
}

// =============================================================================
// Generate all generic trait tests via macro
// =============================================================================

generate_trait_tests!(GameSession);

// =============================================================================
// Struct-Specific CompressAs Tests with Overrides
// =============================================================================

#[test]
fn test_compress_as_overrides_start_time() {
    let player = Pubkey::new_unique();

    let record = GameSession {
        compression_info: CompressionInfo::default(),
        session_id: 1,
        player,
        game_type: "test game".to_string(),
        start_time: 100,
        end_time: Some(200),
        score: 50,
    };

    let compressed = record.compress_as();
    assert_eq!(
        compressed.start_time, 0,
        "compress_as should override start_time to 0"
    );
}

#[test]
fn test_compress_as_overrides_end_time() {
    let player = Pubkey::new_unique();

    let record = GameSession {
        compression_info: CompressionInfo::default(),
        session_id: 1,
        player,
        game_type: "test game".to_string(),
        start_time: 100,
        end_time: Some(200),
        score: 50,
    };

    let compressed = record.compress_as();
    assert_eq!(
        compressed.end_time, None,
        "compress_as should override end_time to None"
    );
}

#[test]
fn test_compress_as_overrides_score() {
    let player = Pubkey::new_unique();

    let record = GameSession {
        compression_info: CompressionInfo::default(),
        session_id: 1,
        player,
        game_type: "test game".to_string(),
        start_time: 100,
        end_time: Some(200),
        score: 50,
    };

    let compressed = record.compress_as();
    assert_eq!(
        compressed.score, 0,
        "compress_as should override score to 0"
    );
}

#[test]
fn test_compress_as_preserves_session_id() {
    let player = Pubkey::new_unique();
    let session_id = 999u64;

    let record = GameSession {
        compression_info: CompressionInfo::default(),
        session_id,
        player,
        game_type: "test game".to_string(),
        start_time: 100,
        end_time: Some(200),
        score: 50,
    };

    let compressed = record.compress_as();
    assert_eq!(
        compressed.session_id, session_id,
        "compress_as should preserve session_id"
    );
}

#[test]
fn test_compress_as_preserves_player() {
    let player = Pubkey::new_unique();

    let record = GameSession {
        compression_info: CompressionInfo::default(),
        session_id: 1,
        player,
        game_type: "test game".to_string(),
        start_time: 100,
        end_time: Some(200),
        score: 50,
    };

    let compressed = record.compress_as();
    assert_eq!(
        compressed.player, player,
        "compress_as should preserve player"
    );
}

#[test]
fn test_compress_as_preserves_game_type() {
    let player = Pubkey::new_unique();
    let game_type = "custom game".to_string();

    let record = GameSession {
        compression_info: CompressionInfo::default(),
        session_id: 1,
        player,
        game_type: game_type.clone(),
        start_time: 100,
        end_time: Some(200),
        score: 50,
    };

    let compressed = record.compress_as();
    assert_eq!(
        compressed.game_type, game_type,
        "compress_as should preserve game_type"
    );
}

// =============================================================================
// Struct-Specific DataHasher Tests
// =============================================================================

#[test]
fn test_hash_differs_for_different_session_id() {
    let player = Pubkey::new_unique();

    let record1 = GameSession {
        compression_info: CompressionInfo::compressed(),
        session_id: 1,
        player,
        game_type: "test game".to_string(),
        start_time: 100,
        end_time: Some(200),
        score: 50,
    };

    let record2 = GameSession {
        compression_info: CompressionInfo::compressed(),
        session_id: 2,
        player,
        game_type: "test game".to_string(),
        start_time: 100,
        end_time: Some(200),
        score: 50,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different session_id should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_player() {
    let record1 = GameSession {
        compression_info: CompressionInfo::compressed(),
        session_id: 1,
        player: Pubkey::new_unique(),
        game_type: "test game".to_string(),
        start_time: 100,
        end_time: Some(200),
        score: 50,
    };

    let record2 = GameSession {
        compression_info: CompressionInfo::compressed(),
        session_id: 1,
        player: Pubkey::new_unique(),
        game_type: "test game".to_string(),
        start_time: 100,
        end_time: Some(200),
        score: 50,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different player should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_game_type() {
    let player = Pubkey::new_unique();

    let record1 = GameSession {
        compression_info: CompressionInfo::compressed(),
        session_id: 1,
        player,
        game_type: "game1".to_string(),
        start_time: 100,
        end_time: Some(200),
        score: 50,
    };

    let record2 = GameSession {
        compression_info: CompressionInfo::compressed(),
        session_id: 1,
        player,
        game_type: "game2".to_string(),
        start_time: 100,
        end_time: Some(200),
        score: 50,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different game_type should produce different hash"
    );
}

// =============================================================================
// Pack/Unpack Tests (struct-specific, cannot be generic)
// =============================================================================

#[test]
fn test_packed_struct_has_u8_player() {
    // Verify PackedGameSession has the expected structure
    // The Packed struct uses the same field name but changes type to u8
    let packed = PackedGameSession {
        session_id: 1,
        player: 0,
        game_type: "test".to_string(),
        start_time: 100,
        end_time: Some(200),
        score: 50,
    };

    assert_eq!(packed.player, 0u8);
    assert_eq!(packed.session_id, 1u64);
}

#[test]
fn test_pack_converts_pubkey_to_index() {
    let player = Pubkey::new_unique();
    let record = GameSession {
        compression_info: CompressionInfo::compressed(),
        session_id: 1,
        player,
        game_type: "test game".to_string(),
        start_time: 100,
        end_time: Some(200),
        score: 50,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts).unwrap();

    // The player should have been added to packed_accounts
    // and packed.player should be the index (0 for first pubkey)
    assert_eq!(packed.player, 0u8);
    assert_eq!(packed.session_id, 1);

    let mut packed_accounts = PackedAccounts::default();
    packed_accounts.insert_or_get(Pubkey::new_unique());
    let packed = record.pack(&mut packed_accounts).unwrap();

    // The player should have been added to packed_accounts
    // and packed.player should be the index (1 for second pubkey)
    assert_eq!(packed.player, 1u8);
    assert_eq!(packed.session_id, 1);
}

#[test]
fn test_pack_reuses_same_pubkey_index() {
    let player = Pubkey::new_unique();

    let record1 = GameSession {
        compression_info: CompressionInfo::compressed(),
        session_id: 1,
        player,
        game_type: "game1".to_string(),
        start_time: 100,
        end_time: Some(200),
        score: 50,
    };

    let record2 = GameSession {
        compression_info: CompressionInfo::compressed(),
        session_id: 2,
        player,
        game_type: "game2".to_string(),
        start_time: 100,
        end_time: Some(200),
        score: 50,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = record1.pack(&mut packed_accounts).unwrap();
    let packed2 = record2.pack(&mut packed_accounts).unwrap();

    // Same pubkey should get same index
    assert_eq!(
        packed1.player, packed2.player,
        "same pubkey should produce same index"
    );
}

#[test]
fn test_pack_different_pubkeys_get_different_indices() {
    let record1 = GameSession {
        compression_info: CompressionInfo::compressed(),
        session_id: 1,
        player: Pubkey::new_unique(),
        game_type: "game1".to_string(),
        start_time: 100,
        end_time: Some(200),
        score: 50,
    };

    let record2 = GameSession {
        compression_info: CompressionInfo::compressed(),
        session_id: 2,
        player: Pubkey::new_unique(),
        game_type: "game2".to_string(),
        start_time: 100,
        end_time: Some(200),
        score: 50,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = record1.pack(&mut packed_accounts).unwrap();
    let packed2 = record2.pack(&mut packed_accounts).unwrap();

    // Different pubkeys should get different indices
    assert_ne!(
        packed1.player, packed2.player,
        "different pubkeys should produce different indices"
    );
}

#[test]
fn test_pack_stores_pubkeys_in_packed_accounts() {
    let player1 = Pubkey::new_unique();
    let player2 = Pubkey::new_unique();

    let record1 = GameSession {
        compression_info: CompressionInfo::compressed(),
        session_id: 1,
        player: player1,
        game_type: "game1".to_string(),
        start_time: 100,
        end_time: Some(200),
        score: 50,
    };

    let record2 = GameSession {
        compression_info: CompressionInfo::compressed(),
        session_id: 2,
        player: player2,
        game_type: "game2".to_string(),
        start_time: 100,
        end_time: Some(200),
        score: 50,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = record1.pack(&mut packed_accounts).unwrap();
    let packed2 = record2.pack(&mut packed_accounts).unwrap();

    // Verify pubkeys are stored and retrievable
    let stored_pubkeys = packed_accounts.packed_pubkeys();
    assert_eq!(stored_pubkeys.len(), 2, "should have 2 pubkeys stored");
    assert_eq!(
        stored_pubkeys[packed1.player as usize],
        player1.to_bytes(),
        "first pubkey should match"
    );
    assert_eq!(
        stored_pubkeys[packed2.player as usize],
        player2.to_bytes(),
        "second pubkey should match"
    );
}
