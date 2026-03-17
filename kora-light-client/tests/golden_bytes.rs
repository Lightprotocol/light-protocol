//! Golden byte cross-verification tests.
//!
//! These tests verify that kora-light-client produces byte-identical instruction
//! data to what the on-chain program expects, by comparing against the known
//! Borsh serialization format.
//!
//! IMPORTANT FINDING:
//! Kora's existing raw-byte builder (`instruction_builder.rs`) uses a DIFFERENT
//! output data format than the current on-chain `MultiTokenTransferOutputData`:
//!
//! Kora's format (12 bytes per output):
//!   owner: u8, amount: u64, lamports: Option<u64>=None, merkle_tree_index: u8, tlv: Option=None
//!
//! On-chain format (13 bytes per output):
//!   owner: u8, amount: u64, has_delegate: bool, delegate: u8, mint: u8, version: u8
//!
//! This crate uses the on-chain format (13 bytes) which matches the source at
//! `program-libs/token-interface/src/instructions/transfer2/instruction_data.rs`.
//! When Kora integrates this crate, its output format will change to match the
//! current on-chain program. If the deployed program uses an older format,
//! this needs investigation.

use borsh::BorshSerialize;
use kora_light_client::types::*;

/// Verify Transfer2 header serialization matches Kora's byte layout.
///
/// Kora builds the header as:
/// [discriminator(1), with_tx_hash(1), with_lamports_change(1),
///  lamports_change_tree(1), lamports_change_owner(1), output_queue(1),
///  max_top_up(2), cpi_context_option(1), compressions_option(1),
///  proof_option(1), proof_bytes(128)]
#[test]
fn test_transfer2_header_matches_kora_format() {
    let proof = CompressedProof {
        a: [0xAA; 32],
        b: [0xBB; 64],
        c: [0xCC; 32],
    };

    let transfer2 = CompressedTokenInstructionDataTransfer2 {
        with_transaction_hash: false,
        with_lamports_change_account_merkle_tree_index: false,
        lamports_change_account_merkle_tree_index: 0,
        lamports_change_account_owner_index: 0,
        output_queue: 3,
        max_top_up: 5000,
        cpi_context: None,
        compressions: None,
        proof: Some(proof),
        in_token_data: Vec::new(),
        out_token_data: Vec::new(),
        in_lamports: None,
        out_lamports: None,
        in_tlv: None,
        out_tlv: None,
    };

    let data = borsh::to_vec(&transfer2).unwrap();

    // Verify header bytes match Kora's manual serialization
    assert_eq!(data[0], 0, "with_transaction_hash = false");
    assert_eq!(data[1], 0, "with_lamports_change = false");
    assert_eq!(data[2], 0, "lamports_change_tree_index = 0");
    assert_eq!(data[3], 0, "lamports_change_owner_index = 0");
    assert_eq!(data[4], 3, "output_queue = 3");
    assert_eq!(&data[5..7], &5000u16.to_le_bytes(), "max_top_up = 5000");
    assert_eq!(data[7], 0, "cpi_context = None");
    assert_eq!(data[8], 0, "compressions = None");
    assert_eq!(data[9], 1, "proof = Some");

    // Proof bytes (128 bytes starting at offset 10)
    assert_eq!(&data[10..42], &[0xAA; 32], "proof.a");
    assert_eq!(&data[42..106], &[0xBB; 64], "proof.b");
    assert_eq!(&data[106..138], &[0xCC; 32], "proof.c");

    // Empty vecs for in_token_data and out_token_data
    assert_eq!(
        &data[138..142],
        &0u32.to_le_bytes(),
        "in_token_data len = 0"
    );
    assert_eq!(
        &data[142..146],
        &0u32.to_le_bytes(),
        "out_token_data len = 0"
    );

    // Trailing None options
    assert_eq!(data[146], 0, "in_lamports = None");
    assert_eq!(data[147], 0, "out_lamports = None");
    assert_eq!(data[148], 0, "in_tlv = None");
    assert_eq!(data[149], 0, "out_tlv = None");

    assert_eq!(data.len(), 150, "Total header size with empty vecs");
}

/// Verify MultiInputTokenDataWithContext serialization matches Kora's
/// `serialize_input_token_data` byte layout (22 bytes per input).
#[test]
fn test_input_token_data_matches_kora_format() {
    let input = MultiInputTokenDataWithContext {
        owner: 4, // owner_index
        amount: 1_000_000,
        has_delegate: false,
        delegate: 0,
        mint: 3, // mint_index
        version: 0,
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: 0,
            queue_pubkey_index: 1,
            leaf_index: 42,
            prove_by_index: false,
        },
        root_index: 7,
    };

    let bytes = borsh::to_vec(&input).unwrap();
    assert_eq!(bytes.len(), 22, "Input token data must be 22 bytes");

    // Verify byte-by-byte against Kora's serialize_input_token_data
    assert_eq!(bytes[0], 4, "owner index");
    assert_eq!(&bytes[1..9], &1_000_000u64.to_le_bytes(), "amount");
    assert_eq!(bytes[9], 0, "has_delegate = false");
    assert_eq!(bytes[10], 0, "delegate index");
    assert_eq!(bytes[11], 3, "mint index");
    assert_eq!(bytes[12], 0, "version");
    // Merkle context
    assert_eq!(bytes[13], 0, "merkle_tree_pubkey_index");
    assert_eq!(bytes[14], 1, "queue_pubkey_index");
    assert_eq!(&bytes[15..19], &42u32.to_le_bytes(), "leaf_index");
    assert_eq!(bytes[19], 0, "prove_by_index = false");
    // Root index
    assert_eq!(&bytes[20..22], &7u16.to_le_bytes(), "root_index");
}

/// Verify MultiTokenTransferOutputData serialization.
///
/// NOTE: This is 13 bytes per output (on-chain format).
/// Kora's raw builder uses 12 bytes (different format — see module doc).
#[test]
fn test_output_token_data_on_chain_format() {
    let output = MultiTokenTransferOutputData {
        owner: 5, // destination_index
        amount: 500_000,
        has_delegate: false,
        delegate: 0,
        mint: 3, // mint_index
        version: 0,
    };

    let bytes = borsh::to_vec(&output).unwrap();
    assert_eq!(
        bytes.len(),
        13,
        "Output token data must be 13 bytes (on-chain format)"
    );

    assert_eq!(bytes[0], 5, "owner index");
    assert_eq!(&bytes[1..9], &500_000u64.to_le_bytes(), "amount");
    assert_eq!(bytes[9], 0, "has_delegate = false");
    assert_eq!(bytes[10], 0, "delegate index");
    assert_eq!(bytes[11], 3, "mint index");
    assert_eq!(bytes[12], 0, "version");
}

/// Verify the full discriminator + struct serialization for Transfer2.
///
/// This is how the instruction data is actually built:
/// [TRANSFER2_DISCRIMINATOR(1 byte)] + [borsh-serialized struct]
#[test]
fn test_full_instruction_data_format() {
    let input = MultiInputTokenDataWithContext {
        owner: 4,
        amount: 1_000_000,
        has_delegate: false,
        delegate: 0,
        mint: 3,
        version: 0,
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: 0,
            queue_pubkey_index: 1,
            leaf_index: 42,
            prove_by_index: false,
        },
        root_index: 7,
    };

    let output = MultiTokenTransferOutputData {
        owner: 5,
        amount: 1_000_000,
        has_delegate: false,
        delegate: 0,
        mint: 3,
        version: 0,
    };

    let proof = CompressedProof {
        a: [1u8; 32],
        b: [2u8; 64],
        c: [3u8; 32],
    };

    let transfer2 = CompressedTokenInstructionDataTransfer2 {
        with_transaction_hash: false,
        with_lamports_change_account_merkle_tree_index: false,
        lamports_change_account_merkle_tree_index: 0,
        lamports_change_account_owner_index: 0,
        output_queue: 0,
        max_top_up: u16::MAX,
        cpi_context: None,
        compressions: None,
        proof: Some(proof),
        in_token_data: vec![input],
        out_token_data: vec![output],
        in_lamports: None,
        out_lamports: None,
        in_tlv: None,
        out_tlv: None,
    };

    // Build as instruction does: discriminator + borsh
    let mut data = vec![101u8]; // TRANSFER2_DISCRIMINATOR
    transfer2.serialize(&mut data).unwrap();

    assert_eq!(data[0], 101, "Discriminator");

    // Header: 10 bytes (5 bools/u8s + u16 + 2 option tags)
    // Proof: 1 (Some) + 128 bytes
    // in_token_data: 4 (len) + 22 (one input)
    // out_token_data: 4 (len) + 13 (one output)
    // Trailing: 4 None option tags
    // Total: 1 + 10 + 129 + 26 + 17 + 4 = 187
    let expected_len = 1 + 7 + 2 + 1 + 128 + 4 + 22 + 4 + 13 + 4;
    assert_eq!(data.len(), expected_len, "Total instruction data length");
}

/// Verify Compression struct serialization (16 bytes).
#[test]
fn test_compression_serialization() {
    let decompress = Compression::decompress_spl(
        500_000, // amount
        2,       // mint index
        4,       // recipient index
        5,       // pool_account_index
        0,       // pool_index
        254,     // bump
        6,       // decimals
    );

    let bytes = borsh::to_vec(&decompress).unwrap();
    assert_eq!(bytes.len(), 16, "Compression struct is 16 bytes");

    assert_eq!(bytes[0], 1, "mode = Decompress (variant 1)");
    assert_eq!(&bytes[1..9], &500_000u64.to_le_bytes(), "amount");
    assert_eq!(bytes[9], 2, "mint index");
    assert_eq!(bytes[10], 4, "recipient index");
    assert_eq!(bytes[11], 0, "authority (not used for decompress)");
    assert_eq!(bytes[12], 5, "pool_account_index");
    assert_eq!(bytes[13], 0, "pool_index");
    assert_eq!(bytes[14], 254, "bump");
    assert_eq!(bytes[15], 6, "decimals");
}

/// Verify CompressedOnlyExtensionInstructionData serialization.
#[test]
fn test_compressed_only_extension_serialization() {
    let ext = CompressedOnlyExtensionInstructionData {
        delegated_amount: 0,
        withheld_transfer_fee: 0,
        is_frozen: false,
        compression_index: 0,
        is_ata: true,
        bump: 255,
        owner_index: 3,
    };

    let bytes = borsh::to_vec(&ext).unwrap();
    // u64 + u64 + bool + u8 + bool + u8 + u8 = 8+8+1+1+1+1+1 = 21
    assert_eq!(bytes.len(), 21);
}

/// Verify ExtensionInstructionData enum discriminators.
#[test]
fn test_extension_enum_discriminators() {
    // Placeholder0 = variant 0
    let bytes = borsh::to_vec(&ExtensionInstructionData::Placeholder0).unwrap();
    assert_eq!(bytes[0], 0, "Placeholder0 discriminator");

    // CompressedOnly = variant 31
    let ext = ExtensionInstructionData::CompressedOnly(CompressedOnlyExtensionInstructionData {
        delegated_amount: 0,
        withheld_transfer_fee: 0,
        is_frozen: false,
        compression_index: 0,
        is_ata: false,
        bump: 0,
        owner_index: 0,
    });
    let bytes = borsh::to_vec(&ext).unwrap();
    assert_eq!(bytes[0], 31, "CompressedOnly discriminator = 31");

    // Compressible = variant 32
    let ext = ExtensionInstructionData::Compressible(CompressionInfo {
        config_account_version: 1,
        compress_to_pubkey: 0,
        account_version: 0,
        lamports_per_write: 766,
        compression_authority: [0u8; 32],
        rent_sponsor: [0u8; 32],
        last_claimed_slot: 0,
    });
    let bytes = borsh::to_vec(&ext).unwrap();
    assert_eq!(bytes[0], 32, "Compressible discriminator = 32");
}

/// Verify the discriminator + Transfer2 data round-trip (serialize → deserialize).
#[test]
fn test_transfer2_roundtrip() {
    let proof = CompressedProof {
        a: [0x11; 32],
        b: [0x22; 64],
        c: [0x33; 32],
    };

    let original = CompressedTokenInstructionDataTransfer2 {
        with_transaction_hash: false,
        with_lamports_change_account_merkle_tree_index: false,
        lamports_change_account_merkle_tree_index: 0,
        lamports_change_account_owner_index: 0,
        output_queue: 2,
        max_top_up: 5000,
        cpi_context: None,
        compressions: Some(vec![Compression::decompress(1000, 1, 3)]),
        proof: Some(proof),
        in_token_data: vec![MultiInputTokenDataWithContext {
            owner: 4,
            amount: 2000,
            has_delegate: false,
            delegate: 0,
            mint: 1,
            version: 0,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: 0,
                queue_pubkey_index: 2,
                leaf_index: 100,
                prove_by_index: true,
            },
            root_index: 5,
        }],
        out_token_data: vec![MultiTokenTransferOutputData {
            owner: 4,
            amount: 1000,
            has_delegate: false,
            delegate: 0,
            mint: 1,
            version: 0,
        }],
        in_lamports: None,
        out_lamports: None,
        in_tlv: None,
        out_tlv: None,
    };

    let bytes = borsh::to_vec(&original).unwrap();
    let deserialized: CompressedTokenInstructionDataTransfer2 = borsh::from_slice(&bytes).unwrap();

    // Verify round-trip
    assert_eq!(deserialized.output_queue, 2);
    assert_eq!(deserialized.max_top_up, 5000);
    assert!(deserialized.proof.is_some());
    assert_eq!(deserialized.in_token_data.len(), 1);
    assert_eq!(deserialized.in_token_data[0].amount, 2000);
    assert_eq!(deserialized.out_token_data.len(), 1);
    assert_eq!(deserialized.out_token_data[0].amount, 1000);
    assert!(deserialized.compressions.is_some());
    let compressions = deserialized.compressions.unwrap();
    assert_eq!(compressions.len(), 1);
    assert_eq!(compressions[0].mode, CompressionMode::Decompress);
    assert_eq!(compressions[0].amount, 1000);
}
