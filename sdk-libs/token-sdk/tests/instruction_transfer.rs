use light_token::instruction::{Transfer, LIGHT_TOKEN_PROGRAM_ID};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

/// Test Transfer instruction with no max_top_up or fee_payer.
/// Authority is readonly signer since it doesn't need to pay for top-ups.
#[test]
fn test_transfer_basic() {
    let source = Pubkey::new_from_array([1u8; 32]);
    let destination = Pubkey::new_from_array([2u8; 32]);
    let authority = Pubkey::new_from_array([3u8; 32]);

    let instruction = Transfer {
        source,
        destination,
        amount: 100,
        authority,
        max_top_up: None,
        fee_payer: None,
    }
    .instruction()
    .expect("Failed to create instruction");

    // Hardcoded expected instruction
    // - authority is readonly (no max_top_up)
    // - data: discriminator (3) + amount (100 as le u64) = 9 bytes
    let expected = Instruction {
        program_id: LIGHT_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(source, false),      // source: writable, not signer
            AccountMeta::new(destination, false), // destination: writable, not signer
            AccountMeta::new_readonly(authority, true), // authority: readonly, signer
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program: readonly, not signer
        ],
        data: vec![
            3u8, // Transfer discriminator
            100, 0, 0, 0, 0, 0, 0, 0, // amount: 100 as little-endian u64
        ],
    };

    assert_eq!(
        instruction, expected,
        "Transfer instruction should match expected"
    );
}

/// Test Transfer instruction with max_top_up set (no fee_payer).
/// Authority becomes writable to pay for potential top-ups.
/// Data includes max_top_up as 2 extra bytes.
#[test]
fn test_transfer_with_max_top_up() {
    let source = Pubkey::new_from_array([1u8; 32]);
    let destination = Pubkey::new_from_array([2u8; 32]);
    let authority = Pubkey::new_from_array([3u8; 32]);

    let instruction = Transfer {
        source,
        destination,
        amount: 100,
        authority,
        max_top_up: Some(500),
        fee_payer: None,
    }
    .instruction()
    .expect("Failed to create instruction");

    // Hardcoded expected instruction
    // - authority is writable (max_top_up set, no fee_payer -> authority pays)
    // - data: discriminator (3) + amount (8 bytes) + max_top_up (2 bytes) = 11 bytes
    let expected = Instruction {
        program_id: LIGHT_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(source, false),      // source: writable, not signer
            AccountMeta::new(destination, false), // destination: writable, not signer
            AccountMeta::new(authority, true),    // authority: writable, signer (pays for top-ups)
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program: readonly, not signer
        ],
        data: vec![
            3u8, // Transfer discriminator
            100, 0, 0, 0, 0, 0, 0, 0, // amount: 100 as little-endian u64
            244, 1, // max_top_up: 500 as little-endian u16
        ],
    };

    assert_eq!(
        instruction, expected,
        "Transfer instruction with max_top_up should match expected"
    );
}

/// Test Transfer instruction with fee_payer set (no max_top_up).
/// Fee_payer is added as 5th account. Authority remains readonly.
#[test]
fn test_transfer_with_fee_payer() {
    let source = Pubkey::new_from_array([1u8; 32]);
    let destination = Pubkey::new_from_array([2u8; 32]);
    let authority = Pubkey::new_from_array([3u8; 32]);
    let fee_payer = Pubkey::new_from_array([4u8; 32]);

    let instruction = Transfer {
        source,
        destination,
        amount: 100,
        authority,
        max_top_up: None,
        fee_payer: Some(fee_payer),
    }
    .instruction()
    .expect("Failed to create instruction");

    // Hardcoded expected instruction
    // - authority is readonly (fee_payer pays instead)
    // - fee_payer is 5th account: writable, signer
    // - data: discriminator (3) + amount (8 bytes) = 9 bytes (no max_top_up)
    let expected = Instruction {
        program_id: LIGHT_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(source, false),      // source: writable, not signer
            AccountMeta::new(destination, false), // destination: writable, not signer
            AccountMeta::new_readonly(authority, true), // authority: readonly, signer
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program: readonly, not signer
            AccountMeta::new(fee_payer, true),                   // fee_payer: writable, signer
        ],
        data: vec![
            3u8, // Transfer discriminator
            100, 0, 0, 0, 0, 0, 0, 0, // amount: 100 as little-endian u64
        ],
    };

    assert_eq!(
        instruction, expected,
        "Transfer instruction with fee_payer should match expected"
    );
}

/// Test Transfer instruction with both max_top_up and fee_payer set.
/// Authority is readonly (fee_payer pays for top-ups).
/// Data includes max_top_up. Fee_payer is 5th account.
#[test]
fn test_transfer_with_max_top_up_and_fee_payer() {
    let source = Pubkey::new_from_array([1u8; 32]);
    let destination = Pubkey::new_from_array([2u8; 32]);
    let authority = Pubkey::new_from_array([3u8; 32]);
    let fee_payer = Pubkey::new_from_array([4u8; 32]);

    let instruction = Transfer {
        source,
        destination,
        amount: 100,
        authority,
        max_top_up: Some(500),
        fee_payer: Some(fee_payer),
    }
    .instruction()
    .expect("Failed to create instruction");

    // Hardcoded expected instruction
    // - authority is readonly (fee_payer pays instead, even with max_top_up)
    // - fee_payer is 5th account: writable, signer
    // - data: discriminator (3) + amount (8 bytes) + max_top_up (2 bytes) = 11 bytes
    let expected = Instruction {
        program_id: LIGHT_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(source, false),      // source: writable, not signer
            AccountMeta::new(destination, false), // destination: writable, not signer
            AccountMeta::new_readonly(authority, true), // authority: readonly, signer
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program: readonly, not signer
            AccountMeta::new(fee_payer, true),                   // fee_payer: writable, signer
        ],
        data: vec![
            3u8, // Transfer discriminator
            100, 0, 0, 0, 0, 0, 0, 0, // amount: 100 as little-endian u64
            244, 1, // max_top_up: 500 as little-endian u16
        ],
    };

    assert_eq!(
        instruction, expected,
        "Transfer instruction with max_top_up and fee_payer should match expected"
    );
}
