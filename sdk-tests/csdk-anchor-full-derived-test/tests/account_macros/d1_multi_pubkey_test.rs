//! D1 Tests: MultiPubkeyRecord trait derive tests
//!
//! Tests each trait derived by `LightAccount` macro for `MultiPubkeyRecord`:
//! - LightHasherSha -> DataHasher + ToByteArray
//! - LightDiscriminator -> LIGHT_DISCRIMINATOR constant
//! - Compressible -> HasCompressionInfo + CompressAs + Size + CompressedInitSpace
//! - CompressiblePack -> Pack + Unpack + PackedMultiPubkeyRecord

use csdk_anchor_full_derived_test::{MultiPubkeyRecord, PackedMultiPubkeyRecord};
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

impl CompressibleTestFactory for MultiPubkeyRecord {
    fn with_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::default(),
            owner: Pubkey::new_unique(),
            delegate: Pubkey::new_unique(),
            authority: Pubkey::new_unique(),
            amount: 0,
        }
    }

    fn without_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::compressed(),
            owner: Pubkey::new_unique(),
            delegate: Pubkey::new_unique(),
            authority: Pubkey::new_unique(),
            amount: 0,
        }
    }
}

// =============================================================================
// Generate all generic trait tests via macro
// =============================================================================

generate_trait_tests!(MultiPubkeyRecord);

// =============================================================================
// Struct-Specific CompressAs Tests
// =============================================================================

#[test]
fn test_compress_as_preserves_other_fields() {
    let owner = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let amount = 999u64;

    let record = MultiPubkeyRecord {
        compression_info: CompressionInfo::default(),
        owner,
        delegate,
        authority,
        amount,
    };

    let compressed = record.compress_as();
    assert_eq!(compressed.owner, owner);
    assert_eq!(compressed.delegate, delegate);
    assert_eq!(compressed.authority, authority);
    assert_eq!(compressed.amount, amount);
}

#[test]
fn test_compress_as_when_compression_info_already_none() {
    let owner = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let amount = 123u64;

    let record = MultiPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate,
        authority,
        amount,
    };

    let compressed = record.compress_as();

    // Should still work and preserve fields
    assert_eq!(
        compressed.compression_info.state,
        light_sdk::compressible::CompressionState::Compressed
    );
    assert_eq!(compressed.owner, owner);
    assert_eq!(compressed.delegate, delegate);
    assert_eq!(compressed.authority, authority);
    assert_eq!(compressed.amount, amount);
}

// =============================================================================
// Struct-Specific DataHasher Tests
// =============================================================================

#[test]
fn test_hash_differs_for_different_amount() {
    let owner = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();
    let authority = Pubkey::new_unique();

    let record1 = MultiPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate,
        authority,
        amount: 1,
    };

    let record2 = MultiPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate,
        authority,
        amount: 2,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different amount should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_owner() {
    let delegate = Pubkey::new_unique();
    let authority = Pubkey::new_unique();

    let record1 = MultiPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        delegate,
        authority,
        amount: 100,
    };

    let record2 = MultiPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        delegate,
        authority,
        amount: 100,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different owner should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_delegate() {
    let owner = Pubkey::new_unique();
    let authority = Pubkey::new_unique();

    let record1 = MultiPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: Pubkey::new_unique(),
        authority,
        amount: 100,
    };

    let record2 = MultiPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: Pubkey::new_unique(),
        authority,
        amount: 100,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different delegate should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_authority() {
    let owner = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();

    let record1 = MultiPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate,
        authority: Pubkey::new_unique(),
        amount: 100,
    };

    let record2 = MultiPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate,
        authority: Pubkey::new_unique(),
        amount: 100,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different authority should produce different hash"
    );
}

// =============================================================================
// Pack/Unpack Tests (struct-specific, cannot be generic)
// =============================================================================

#[test]
fn test_packed_struct_has_u8_indices() {
    // Verify PackedMultiPubkeyRecord has three u8 index fields
    // Note: PackedMultiPubkeyRecord no longer has compression_info field
    let packed = PackedMultiPubkeyRecord {
        owner: 0,
        delegate: 1,
        authority: 2,
        amount: 42,
    };

    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.delegate, 1u8);
    assert_eq!(packed.authority, 2u8);
    assert_eq!(packed.amount, 42u64);
}

#[test]
fn test_pack_converts_all_pubkeys_to_indices() {
    let owner = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();
    let authority = Pubkey::new_unique();

    let record = MultiPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate,
        authority,
        amount: 100,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts).unwrap();

    // All three Pubkeys should have been added and packed should have their indices
    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.delegate, 1u8);
    assert_eq!(packed.authority, 2u8);
    assert_eq!(packed.amount, 100);

    let stored_pubkeys = packed_accounts.packed_pubkeys();
    assert_eq!(stored_pubkeys.len(), 3);
    assert_eq!(stored_pubkeys[0], owner);
    assert_eq!(stored_pubkeys[1], delegate);
    assert_eq!(stored_pubkeys[2], authority);
}

#[test]
fn test_pack_reuses_pubkey_indices() {
    let owner = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();
    let authority = Pubkey::new_unique();

    let record1 = MultiPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate,
        authority,
        amount: 1,
    };

    let record2 = MultiPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate,
        authority,
        amount: 2,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = record1.pack(&mut packed_accounts).unwrap();
    let packed2 = record2.pack(&mut packed_accounts).unwrap();

    // Same pubkeys should get same indices
    assert_eq!(packed1.owner, packed2.owner);
    assert_eq!(packed1.delegate, packed2.delegate);
    assert_eq!(packed1.authority, packed2.authority);

    // Should still only have 3 pubkeys total
    let stored_pubkeys = packed_accounts.packed_pubkeys();
    assert_eq!(stored_pubkeys.len(), 3);
}

#[test]
fn test_pack_different_pubkeys_get_different_indices() {
    let record1 = MultiPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        delegate: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        amount: 1,
    };

    let record2 = MultiPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        delegate: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        amount: 2,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = record1.pack(&mut packed_accounts).unwrap();
    let packed2 = record2.pack(&mut packed_accounts).unwrap();

    // Different pubkeys should get different indices
    assert_ne!(
        packed1.owner, packed2.owner,
        "different owner pubkeys should produce different indices"
    );
    assert_ne!(
        packed1.delegate, packed2.delegate,
        "different delegate pubkeys should produce different indices"
    );
    assert_ne!(
        packed1.authority, packed2.authority,
        "different authority pubkeys should produce different indices"
    );
}

#[test]
fn test_pack_stores_all_pubkeys_in_packed_accounts() {
    let owner1 = Pubkey::new_unique();
    let delegate1 = Pubkey::new_unique();
    let authority1 = Pubkey::new_unique();

    let owner2 = Pubkey::new_unique();
    let delegate2 = Pubkey::new_unique();
    let authority2 = Pubkey::new_unique();

    let record1 = MultiPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner: owner1,
        delegate: delegate1,
        authority: authority1,
        amount: 1,
    };

    let record2 = MultiPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner: owner2,
        delegate: delegate2,
        authority: authority2,
        amount: 2,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = record1.pack(&mut packed_accounts).unwrap();
    let packed2 = record2.pack(&mut packed_accounts).unwrap();

    // Verify pubkeys are stored and retrievable
    let stored_pubkeys = packed_accounts.packed_pubkeys();
    assert_eq!(stored_pubkeys.len(), 6, "should have 6 pubkeys stored");
    assert_eq!(
        stored_pubkeys[packed1.owner as usize], owner1,
        "first record owner should match"
    );
    assert_eq!(
        stored_pubkeys[packed1.delegate as usize], delegate1,
        "first record delegate should match"
    );
    assert_eq!(
        stored_pubkeys[packed1.authority as usize], authority1,
        "first record authority should match"
    );
    assert_eq!(
        stored_pubkeys[packed2.owner as usize], owner2,
        "second record owner should match"
    );
    assert_eq!(
        stored_pubkeys[packed2.delegate as usize], delegate2,
        "second record delegate should match"
    );
    assert_eq!(
        stored_pubkeys[packed2.authority as usize], authority2,
        "second record authority should match"
    );
}
