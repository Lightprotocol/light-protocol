use light_token::instruction::{Transfer, LIGHT_TOKEN_PROGRAM_ID};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

/// Test Transfer instruction without fee_payer.
/// Authority is writable signer since it pays for top-ups.
/// Short format: discriminator (1) + amount (8) = 9 bytes.
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
        fee_payer: None,
    }
    .instruction()
    .expect("Failed to create instruction");

    // Hardcoded expected instruction
    // - authority is writable (no fee_payer -> authority pays for top-ups)
    // - data: discriminator (3) + amount (8 bytes) = 9 bytes (short format, on-chain defaults max_top_up to u16::MAX)
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
        ],
    };

    assert_eq!(
        instruction, expected,
        "Transfer instruction should match expected"
    );
}

/// Test Transfer instruction with fee_payer set.
/// Fee_payer is added as 5th account. Authority is readonly.
/// Short format: discriminator (1) + amount (8) = 9 bytes.
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
        fee_payer: Some(fee_payer),
    }
    .instruction()
    .expect("Failed to create instruction");

    // Hardcoded expected instruction
    // - authority is readonly (fee_payer pays instead)
    // - fee_payer is 5th account: writable, signer
    // - data: discriminator (3) + amount (8 bytes) = 9 bytes (short format)
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
