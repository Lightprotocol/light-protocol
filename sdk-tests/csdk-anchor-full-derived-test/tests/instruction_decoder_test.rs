//! Tests for the InstructionDecoder derive macro.
//!
//! This test demonstrates how to use the `#[derive(InstructionDecoder)]` macro
//! to generate instruction decoders for Anchor programs.

use anchor_lang::Discriminator;
use light_instruction_decoder_derive::InstructionDecoder;
use light_program_test::logging::{DecoderRegistry, InstructionDecoder as InstructionDecoderTrait};
use solana_pubkey::Pubkey;

/// Example instruction enum for testing the derive macro
#[derive(InstructionDecoder)]
#[instruction_decoder(
    program_id = "FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah",
    program_name = "Test Program"
)]
pub enum TestInstruction {
    /// Initialize instruction with no fields
    Initialize,
    /// Create record with owner
    CreateRecord { owner: Pubkey },
    /// Update record with score
    UpdateRecord { score: u64 },
    /// Transfer with amount and destination
    Transfer { amount: u64, destination: Pubkey },
}

#[test]
fn test_instruction_decoder_macro_generates_decoder() {
    // The macro should have generated TestInstructionDecoder struct
    let decoder = TestInstructionDecoder;

    // Test program ID
    let expected_id: Pubkey = "FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah"
        .parse()
        .unwrap();
    assert_eq!(decoder.program_id(), expected_id);

    // Test program name
    assert_eq!(decoder.program_name(), "Test Program");
}

#[test]
fn test_instruction_decoder_can_be_registered() {
    let decoder = TestInstructionDecoder;

    // Create a registry and register our decoder
    let mut registry = DecoderRegistry::new();
    registry.register(Box::new(decoder));

    // Verify the decoder is registered
    let program_id: Pubkey = "FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah"
        .parse()
        .unwrap();
    assert!(registry.has_decoder(&program_id));
}

#[test]
fn test_instruction_decoder_decodes_instructions() {
    use sha2::{Digest, Sha256};

    let decoder = TestInstructionDecoder;

    // Test decoding an instruction with valid discriminator
    // Compute the expected discriminator for "initialize"
    let hash = Sha256::digest(b"global:initialize");
    let discriminator: [u8; 8] = hash[..8].try_into().unwrap();

    let data = discriminator.to_vec();
    // No additional data for Initialize

    let result = decoder.decode(&data, &[]);
    assert!(result.is_some());
    let decoded = result.unwrap();
    assert_eq!(decoded.name, "Initialize");
}

#[test]
fn test_instruction_decoder_returns_none_for_unknown() {
    let decoder = TestInstructionDecoder;

    // Test with invalid discriminator
    let data = [0u8; 16];
    let result = decoder.decode(&data, &[]);
    assert!(result.is_none());
}

#[test]
fn test_instruction_decoder_with_fields() {
    use sha2::{Digest, Sha256};

    let decoder = TestInstructionDecoder;

    // Test decoding CreateRecord instruction
    let hash = Sha256::digest(b"global:create_record");
    let discriminator: [u8; 8] = hash[..8].try_into().unwrap();

    let mut data = discriminator.to_vec();
    // Add dummy owner pubkey data (32 bytes)
    data.extend_from_slice(&[1u8; 32]);

    let result = decoder.decode(&data, &[]);
    assert!(result.is_some());
    let decoded = result.unwrap();
    assert_eq!(decoded.name, "CreateRecord");
    // Should have fields reported
    assert!(!decoded.fields.is_empty());
}

// =============================================================================
// Tests for enhanced InstructionDecoder with accounts and params attributes
// =============================================================================

/// Test that CsdkTestInstructionDecoder decodes CreateTwoMints with correct account names
#[test]
fn test_enhanced_decoder_account_names() {
    use csdk_anchor_full_derived_test::CsdkTestInstructionDecoder;

    let decoder = CsdkTestInstructionDecoder;

    // Verify program ID and name
    let expected_id: Pubkey = "FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah"
        .parse()
        .unwrap();
    assert_eq!(decoder.program_id(), expected_id);
    assert_eq!(decoder.program_name(), "Csdk Anchor Full Derived Test");

    // Use Anchor's generated discriminator for create_two_mints
    let discriminator = csdk_anchor_full_derived_test::instruction::CreateTwoMints::DISCRIMINATOR;

    // Build minimal instruction data (discriminator + enough bytes for params)
    let mut data = discriminator.to_vec();
    // Add dummy params data - enough to pass the discriminator check
    data.extend_from_slice(&[0u8; 200]);

    let result = decoder.decode(&data, &[]);
    assert!(result.is_some(), "Decoder should recognize CreateTwoMints");

    let decoded = result.unwrap();
    assert_eq!(decoded.name, "CreateTwoMints");

    // Verify account names are populated from the accounts struct
    assert!(
        !decoded.account_names.is_empty(),
        "Account names should not be empty"
    );

    // Check specific account names from CreateTwoMints struct
    let expected_account_names = [
        "fee_payer",
        "authority",
        "mint_signer_a",
        "mint_signer_b",
        "cmint_a",
        "cmint_b",
        "compression_config",
        "light_token_compressible_config",
        "rent_sponsor",
        "light_token_program",
        "light_token_cpi_authority",
        "system_program",
    ];

    assert_eq!(
        decoded.account_names.len(),
        expected_account_names.len(),
        "Should have {} account names, got {}",
        expected_account_names.len(),
        decoded.account_names.len()
    );

    for (i, expected_name) in expected_account_names.iter().enumerate() {
        assert_eq!(
            decoded.account_names[i], *expected_name,
            "Account name at index {} should be '{}', got '{}'",
            i, expected_name, decoded.account_names[i]
        );
    }
}

/// Test that CsdkTestInstructionDecoder decodes params with Debug output
#[test]
fn test_enhanced_decoder_params_decoding() {
    use borsh::BorshSerialize;
    use csdk_anchor_full_derived_test::{
        instruction_accounts::CreateTwoMintsParams, CsdkTestInstructionDecoder,
    };
    use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
    use light_compressible::CreateAccountsProof;
    use light_sdk_types::instruction::PackedAddressTreeInfo;

    let decoder = CsdkTestInstructionDecoder;

    // Use Anchor's generated discriminator for create_two_mints
    let discriminator = csdk_anchor_full_derived_test::instruction::CreateTwoMints::DISCRIMINATOR;

    // Build instruction data with actual serialized params
    let params = CreateTwoMintsParams {
        create_accounts_proof: CreateAccountsProof {
            proof: ValidityProof(None),
            address_tree_info: PackedAddressTreeInfo {
                address_merkle_tree_pubkey_index: 0,
                address_queue_pubkey_index: 0,
                root_index: 0,
            },
            output_state_tree_index: 0,
            state_tree_index: None,
            system_accounts_offset: 0,
        },
        mint_signer_a_bump: 254,
        mint_signer_b_bump: 255,
    };

    let mut data = discriminator.to_vec();
    params.serialize(&mut data).unwrap();

    let result = decoder.decode(&data, &[]);
    assert!(result.is_some(), "Decoder should recognize CreateTwoMints");

    let decoded = result.unwrap();
    assert_eq!(decoded.name, "CreateTwoMints");

    // Verify params are decoded
    assert!(
        !decoded.fields.is_empty(),
        "Fields should contain decoded params"
    );

    // The params field contains the decoded parameter data
    let params_field = decoded.fields.first();
    assert!(params_field.is_some(), "Should have a params field");

    let params_value = &params_field.unwrap().value;
    assert!(
        params_value.contains("mint_signer_a_bump: 254"),
        "Params should contain 'mint_signer_a_bump: 254', got: {}",
        params_value
    );
    assert!(
        params_value.contains("mint_signer_b_bump: 255"),
        "Params should contain 'mint_signer_b_bump: 255', got: {}",
        params_value
    );
}

/// Test that unknown instructions return None (fallback behavior)
#[test]
fn test_enhanced_decoder_unknown_instruction() {
    use csdk_anchor_full_derived_test::CsdkTestInstructionDecoder;

    let decoder = CsdkTestInstructionDecoder;

    // Use an invalid discriminator
    let data = [0u8; 16];
    let result = decoder.decode(&data, &[]);
    assert!(result.is_none(), "Unknown instruction should return None");
}

// =============================================================================
// Tests for #[instruction_decoder] attribute macro
// =============================================================================

/// Test that CsdkAnchorFullDerivedTestInstructionDecoder (from attribute macro) works
/// This decoder is auto-generated by the #[instruction_decoder] attribute on the program module.
#[test]
fn test_attribute_macro_decoder() {
    use csdk_anchor_full_derived_test::CsdkAnchorFullDerivedTestInstructionDecoder;

    let decoder = CsdkAnchorFullDerivedTestInstructionDecoder;

    // Verify program ID uses crate::ID
    let expected_id: Pubkey = "FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah"
        .parse()
        .unwrap();
    assert_eq!(decoder.program_id(), expected_id);

    // Test decoding create_two_mints instruction using Anchor's generated discriminator
    let discriminator = csdk_anchor_full_derived_test::instruction::CreateTwoMints::DISCRIMINATOR;

    let mut data = discriminator.to_vec();
    data.extend_from_slice(&[0u8; 100]); // dummy data

    let result = decoder.decode(&data, &[]);
    assert!(result.is_some());

    let decoded = result.unwrap();
    assert_eq!(decoded.name, "CreateTwoMints");

    // Verify account names are populated
    assert!(
        !decoded.account_names.is_empty(),
        "Should have account names"
    );
    assert!(decoded.account_names.contains(&"fee_payer".to_string()));
}

/// Test that attribute macro decoder has account names for all instructions
#[test]
fn test_attribute_macro_decoder_account_names() {
    use csdk_anchor_full_derived_test::CsdkAnchorFullDerivedTestInstructionDecoder;

    let decoder = CsdkAnchorFullDerivedTestInstructionDecoder;

    // Test create_pdas_and_mint_auto using Anchor's generated discriminator
    let discriminator =
        csdk_anchor_full_derived_test::instruction::CreatePdasAndMintAuto::DISCRIMINATOR;
    let mut data = discriminator.to_vec();
    data.extend_from_slice(&[0u8; 100]);

    let result = decoder.decode(&data, &[]);
    assert!(result.is_some(), "Should decode create_pdas_and_mint_auto");
    let decoded = result.unwrap();

    // Verify specific account names from CreatePdasAndMintAuto struct
    let expected_accounts = [
        "fee_payer",
        "authority",
        "mint_authority",
        "mint_signer",
        "user_record",
        "game_session",
        "mint",
        "vault",
        "vault_authority",
        "user_ata",
        "compression_config",
        "pda_rent_sponsor",
        "light_token_compressible_config",
        "rent_sponsor",
        "light_token_program",
        "light_token_cpi_authority",
        "system_program",
    ];

    assert_eq!(
        decoded.account_names.len(),
        expected_accounts.len(),
        "create_pdas_and_mint_auto should have {} accounts, got {}",
        expected_accounts.len(),
        decoded.account_names.len()
    );

    for (i, expected) in expected_accounts.iter().enumerate() {
        assert_eq!(
            decoded.account_names[i], *expected,
            "Account at index {} should be '{}', got '{}'",
            i, expected, decoded.account_names[i]
        );
    }
}

/// Test that attribute macro decoder handles initialize_pool (AMM test)
#[test]
fn test_attribute_macro_decoder_initialize_pool() {
    use csdk_anchor_full_derived_test::CsdkAnchorFullDerivedTestInstructionDecoder;

    let decoder = CsdkAnchorFullDerivedTestInstructionDecoder;

    // Use Anchor's generated discriminator for initialize_pool
    let discriminator = csdk_anchor_full_derived_test::instruction::InitializePool::DISCRIMINATOR;
    let mut data = discriminator.to_vec();
    data.extend_from_slice(&[0u8; 100]);

    let result = decoder.decode(&data, &[]);
    assert!(result.is_some(), "Should decode initialize_pool");
    let decoded = result.unwrap();

    assert_eq!(decoded.name, "InitializePool");
    assert!(
        !decoded.account_names.is_empty(),
        "InitializePool should have account names"
    );

    // Check for specific AMM accounts
    assert!(
        decoded.account_names.contains(&"creator".to_string()),
        "Should have 'creator' account"
    );
    assert!(
        decoded.account_names.contains(&"pool_state".to_string()),
        "Should have 'pool_state' account"
    );
    assert!(
        decoded.account_names.contains(&"token_0_vault".to_string()),
        "Should have 'token_0_vault' account"
    );
}

/// Test attribute macro decoder with actual serialized instruction data
#[test]
fn test_attribute_macro_decoder_with_instruction_data() {
    use borsh::BorshSerialize;
    use csdk_anchor_full_derived_test::{
        instruction_accounts::CreateTwoMintsParams, CsdkAnchorFullDerivedTestInstructionDecoder,
    };
    use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
    use light_compressible::CreateAccountsProof;
    use light_program_test::logging::InstructionDecoder;
    use light_sdk_types::instruction::PackedAddressTreeInfo;

    let decoder = CsdkAnchorFullDerivedTestInstructionDecoder;

    // Use Anchor's generated discriminator for create_two_mints
    let discriminator = csdk_anchor_full_derived_test::instruction::CreateTwoMints::DISCRIMINATOR;

    // Build instruction data with actual serialized params
    let params = CreateTwoMintsParams {
        create_accounts_proof: CreateAccountsProof {
            proof: ValidityProof(None),
            address_tree_info: PackedAddressTreeInfo {
                address_merkle_tree_pubkey_index: 0,
                address_queue_pubkey_index: 0,
                root_index: 0,
            },
            output_state_tree_index: 0,
            state_tree_index: None,
            system_accounts_offset: 0,
        },
        mint_signer_a_bump: 254,
        mint_signer_b_bump: 255,
    };

    let mut data = discriminator.to_vec();
    params.serialize(&mut data).unwrap();

    println!("Instruction data length: {} bytes", data.len());
    println!("Discriminator: {:?}", &data[0..8]);

    let result = decoder.decode(&data, &[]);
    assert!(result.is_some(), "Should decode create_two_mints");

    let decoded = result.unwrap();
    println!("Decoded instruction: {}", decoded.name);
    println!("Account names: {:?}", decoded.account_names);
    println!("Fields: {:?}", decoded.fields);

    assert_eq!(decoded.name, "CreateTwoMints");

    // Verify account names are correct
    assert_eq!(decoded.account_names.len(), 12);
    assert_eq!(decoded.account_names[0], "fee_payer");
    assert_eq!(decoded.account_names[1], "authority");

    // The attribute macro decodes params - requires Debug impl (compile error if missing)
    assert_eq!(decoded.fields.len(), 1, "Should have 1 field (params)");
    // Field name is now the actual parameter name
    assert_eq!(decoded.fields[0].name, "params");

    // Verify params contain expected values
    let params_value = &decoded.fields[0].value;
    assert!(
        params_value.contains("mint_signer_a_bump: 254"),
        "Params should contain 'mint_signer_a_bump: 254', got: {}",
        params_value
    );
    assert!(
        params_value.contains("mint_signer_b_bump: 255"),
        "Params should contain 'mint_signer_b_bump: 255', got: {}",
        params_value
    );
}

/// Test that InstructionDecoder discriminators match Anchor's DISCRIMINATOR constants.
/// This validates consistency between the InstructionDecoder macro and Anchor's instruction generation.
#[test]
fn test_discriminators_match_anchor_constants() {
    use sha2::{Digest, Sha256};

    // Verify the sha256 computation matches Anchor's DISCRIMINATOR for each instruction
    let instructions: &[(&str, &[u8])] = &[
        (
            "create_two_mints",
            csdk_anchor_full_derived_test::instruction::CreateTwoMints::DISCRIMINATOR,
        ),
        (
            "create_three_mints",
            csdk_anchor_full_derived_test::instruction::CreateThreeMints::DISCRIMINATOR,
        ),
        (
            "create_pdas_and_mint_auto",
            csdk_anchor_full_derived_test::instruction::CreatePdasAndMintAuto::DISCRIMINATOR,
        ),
        (
            "initialize_pool",
            csdk_anchor_full_derived_test::instruction::InitializePool::DISCRIMINATOR,
        ),
    ];

    for (name, anchor_discriminator) in instructions {
        let hash = Sha256::digest(format!("global:{}", name).as_bytes());
        let computed = &hash[..8];

        assert_eq!(
            computed, *anchor_discriminator,
            "Discriminator mismatch for '{}': computed {:?} != anchor {:?}",
            name, computed, anchor_discriminator
        );
    }
}
