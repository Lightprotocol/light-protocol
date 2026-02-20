use light_token::instruction::{Transfer, LIGHT_TOKEN_PROGRAM_ID};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

/// Test Transfer instruction (no max_top_up).
/// Authority is readonly signer; fee_payer is always writable signer.
#[test]
fn test_transfer_basic() {
    let source = Pubkey::new_from_array([1u8; 32]);
    let destination = Pubkey::new_from_array([2u8; 32]);
    let authority = Pubkey::new_from_array([3u8; 32]);
    let fee_payer = Pubkey::new_from_array([4u8; 32]);

    let instruction = Transfer {
        source,
        destination,
        amount: 100,
        authority,
        fee_payer,
    }
    .instruction()
    .expect("Failed to create instruction");

    let expected = Instruction {
        program_id: LIGHT_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(source, false),
            AccountMeta::new(destination, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(Pubkey::default(), false),
            AccountMeta::new(fee_payer, true),
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

/// Test Transfer instruction with max_top_up via builder.
/// max_top_up is appended as 2 extra bytes to instruction data.
#[test]
fn test_transfer_with_max_top_up() {
    let source = Pubkey::new_from_array([1u8; 32]);
    let destination = Pubkey::new_from_array([2u8; 32]);
    let authority = Pubkey::new_from_array([3u8; 32]);
    let fee_payer = Pubkey::new_from_array([4u8; 32]);

    let instruction = Transfer {
        source,
        destination,
        amount: 100,
        authority,
        fee_payer,
    }
    .with_max_top_up(500)
    .instruction()
    .expect("Failed to create instruction");

    let expected = Instruction {
        program_id: LIGHT_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(source, false),
            AccountMeta::new(destination, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(Pubkey::default(), false),
            AccountMeta::new(fee_payer, true),
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
