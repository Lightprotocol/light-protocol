//! D1 Tests: OptionPubkeyRecord trait derive tests
//!
//! Tests each trait derived by `LightAccount` macro for `OptionPubkeyRecord`:
//! - LightHasherSha -> DataHasher + ToByteArray
//! - LightDiscriminator -> LIGHT_DISCRIMINATOR constant
//! - Compressible -> HasCompressionInfo + CompressAs + Size + CompressedInitSpace
//! - CompressiblePack -> Pack + Unpack + PackedOptionPubkeyRecord
//!
//! IMPORTANT: Option<Pubkey> fields are NOT converted to Option<u8> in the packed struct.
//! Only direct Pubkey fields (like `owner: Pubkey`) are converted to u8 indices.
//! Option<Pubkey> fields remain as Option<Pubkey> in the packed struct.

use csdk_anchor_full_derived_test::OptionPubkeyRecord;
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

impl CompressibleTestFactory for OptionPubkeyRecord {
    fn with_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::default(),
            owner: Pubkey::new_unique(),
            delegate: Some(Pubkey::new_unique()),
            close_authority: Some(Pubkey::new_unique()),
            amount: 0,
        }
    }

    fn without_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::compressed(),
            owner: Pubkey::new_unique(),
            delegate: None,
            close_authority: None,
            amount: 0,
        }
    }
}

// =============================================================================
// Generate all generic trait tests via macro
// =============================================================================

generate_trait_tests!(OptionPubkeyRecord);

// =============================================================================
// Struct-Specific CompressAs Tests
// =============================================================================

#[test]
fn test_compress_as_preserves_other_fields() {
    let owner = Pubkey::new_unique();
    let delegate = Some(Pubkey::new_unique());
    let close_authority = Some(Pubkey::new_unique());
    let amount = 999u64;

    let record = OptionPubkeyRecord {
        compression_info: CompressionInfo::default(),
        owner,
        delegate,
        close_authority,
        amount,
    };

    let compressed = record.compress_as();
    assert_eq!(compressed.owner, owner);
    assert_eq!(compressed.delegate, delegate);
    assert_eq!(compressed.close_authority, close_authority);
    assert_eq!(compressed.amount, amount);
}

#[test]
fn test_compress_as_when_compression_info_already_none() {
    let owner = Pubkey::new_unique();
    let delegate = Some(Pubkey::new_unique());
    let close_authority = None;
    let amount = 123u64;

    let record = OptionPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate,
        close_authority,
        amount,
    };

    let compressed = record.compress_as();

    // Should still work and preserve fields    assert_eq!(compressed.owner, owner);
    assert_eq!(compressed.delegate, delegate);
    assert_eq!(compressed.close_authority, close_authority);
    assert_eq!(compressed.amount, amount);
}

// =============================================================================
// Struct-Specific DataHasher Tests
// =============================================================================

#[test]
fn test_hash_differs_for_different_amount() {
    let owner = Pubkey::new_unique();

    let record1 = OptionPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: Some(Pubkey::new_unique()),
        close_authority: None,
        amount: 1,
    };

    let record2 = OptionPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: Some(Pubkey::new_unique()),
        close_authority: None,
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
    let record1 = OptionPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        delegate: None,
        close_authority: Some(Pubkey::new_unique()),
        amount: 100,
    };

    let record2 = OptionPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        delegate: None,
        close_authority: Some(Pubkey::new_unique()),
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

    let record1 = OptionPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: Some(Pubkey::new_unique()),
        close_authority: None,
        amount: 100,
    };

    let record2 = OptionPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: Some(Pubkey::new_unique()),
        close_authority: None,
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
fn test_hash_differs_for_different_close_authority() {
    let owner = Pubkey::new_unique();

    let record1 = OptionPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: None,
        close_authority: Some(Pubkey::new_unique()),
        amount: 100,
    };

    let record2 = OptionPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: None,
        close_authority: Some(Pubkey::new_unique()),
        amount: 100,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different close_authority should produce different hash"
    );
}

// =============================================================================
// Pack/Unpack Tests (struct-specific, cannot be generic)
// =============================================================================

#[test]
fn test_pack_converts_pubkey_fields_to_indices() {
    // Verify that pack() converts Pubkey fields to u8 indices
    // This test checks the Pack trait implementation
    let owner = Pubkey::new_unique();
    let record = OptionPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: None,
        close_authority: None,
        amount: 42,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts).unwrap();

    // The packed struct should have owner as u8 index (0 since it's first pubkey)
    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.delegate, None);
    assert_eq!(packed.close_authority, None);
    assert_eq!(packed.amount, 42u64);
}

#[test]
fn test_pack_converts_pubkey_to_index() {
    let owner = Pubkey::new_unique();
    let record = OptionPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: None,
        close_authority: None,
        amount: 100,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts).unwrap();

    // The owner should have been added and packed.owner should be the index (0 for first pubkey)
    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.delegate, None);
    assert_eq!(packed.close_authority, None);
    assert_eq!(packed.amount, 100);

    let stored_pubkeys = packed_accounts.packed_pubkeys();
    assert_eq!(stored_pubkeys.len(), 1);
    assert_eq!(stored_pubkeys[0], owner);
}

#[test]
fn test_pack_preserves_option_pubkey_as_option_pubkey() {
    // Option<Pubkey> fields are NOT converted to Option<u8>
    // They remain as Option<Pubkey> in the packed struct
    let owner = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();

    let record = OptionPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: Some(delegate),
        close_authority: None,
        amount: 100,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts).unwrap();

    // Direct Pubkey field is converted to u8 index
    assert_eq!(packed.owner, 0u8);
    // Option<Pubkey> stays as Option<Pubkey> - NOT converted to Option<u8>
    assert_eq!(packed.delegate, Some(delegate));
    assert_eq!(packed.close_authority, None);

    // Only the direct Pubkey field (owner) is stored in packed_accounts
    let stored_pubkeys = packed_accounts.packed_pubkeys();
    assert_eq!(stored_pubkeys.len(), 1);
    assert_eq!(stored_pubkeys[0], owner);
}

#[test]
fn test_pack_option_pubkey_none_stays_none() {
    // Option<Pubkey>::None remains None in packed struct
    let owner = Pubkey::new_unique();
    let close_authority = Pubkey::new_unique();

    let record = OptionPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: None,
        close_authority: Some(close_authority),
        amount: 100,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts).unwrap();

    // Direct Pubkey field is converted to u8 index
    assert_eq!(packed.owner, 0u8);
    // Option<Pubkey> fields stay as Option<Pubkey> - NOT converted to Option<u8>
    assert_eq!(packed.delegate, None, "Option::None stays None");
    assert_eq!(
        packed.close_authority,
        Some(close_authority),
        "Option::Some stays Some"
    );

    // Only the direct Pubkey field (owner) is stored in packed_accounts
    let stored_pubkeys = packed_accounts.packed_pubkeys();
    assert_eq!(stored_pubkeys.len(), 1);
    assert_eq!(stored_pubkeys[0], owner);
}

#[test]
fn test_pack_all_option_pubkeys_some() {
    // Tests that Option<Pubkey> fields with Some values are preserved as-is
    let owner = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();
    let close_authority = Pubkey::new_unique();

    let record = OptionPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: Some(delegate),
        close_authority: Some(close_authority),
        amount: 100,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts).unwrap();

    // Direct Pubkey field is converted to u8 index
    assert_eq!(packed.owner, 0u8);
    // Option<Pubkey> fields stay as Option<Pubkey>
    assert_eq!(packed.delegate, Some(delegate));
    assert_eq!(packed.close_authority, Some(close_authority));

    // Only the direct Pubkey field (owner) is stored in packed_accounts
    let stored_pubkeys = packed_accounts.packed_pubkeys();
    assert_eq!(stored_pubkeys.len(), 1);
    assert_eq!(stored_pubkeys[0], owner);
}

#[test]
fn test_pack_all_option_pubkeys_none() {
    let owner = Pubkey::new_unique();

    let record = OptionPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: None,
        close_authority: None,
        amount: 100,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts).unwrap();

    // Only owner should have been added
    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.delegate, None);
    assert_eq!(packed.close_authority, None);

    let stored_pubkeys = packed_accounts.packed_pubkeys();
    assert_eq!(stored_pubkeys.len(), 1);
    assert_eq!(stored_pubkeys[0], owner);
}

#[test]
fn test_pack_reuses_same_pubkey_index_for_direct_fields() {
    // Tests that the same Pubkey in the direct (non-Option) field gets the same index
    let owner = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();

    let record1 = OptionPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: Some(delegate),
        close_authority: None,
        amount: 1,
    };

    let record2 = OptionPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: Some(delegate),
        close_authority: None,
        amount: 2,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = record1.pack(&mut packed_accounts).unwrap();
    let packed2 = record2.pack(&mut packed_accounts).unwrap();

    // Same direct Pubkey field should get same index
    assert_eq!(
        packed1.owner, packed2.owner,
        "same owner should produce same index"
    );
    // Option<Pubkey> fields stay as Option<Pubkey> (not converted to indices)
    assert_eq!(packed1.delegate, packed2.delegate);

    // Only one pubkey stored (owner) since it's the only direct Pubkey field
    let stored_pubkeys = packed_accounts.packed_pubkeys();
    assert_eq!(stored_pubkeys.len(), 1);
}
