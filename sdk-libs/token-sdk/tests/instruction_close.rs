use light_token::instruction::{CloseAccount, LIGHT_TOKEN_PROGRAM_ID, LIGHT_TOKEN_RENT_SPONSOR};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

/// Test CloseAccount instruction with default RENT_SPONSOR.
/// Verifies: program_id, all 4 accounts (pubkeys + writeability + signedness), and data (discriminator 9).
#[test]
fn test_close_account_instruction() {
    // Use deterministic pubkeys for regression testing
    let account = Pubkey::new_from_array([1u8; 32]);
    let destination = Pubkey::new_from_array([2u8; 32]);
    let owner = Pubkey::new_from_array([3u8; 32]);

    let instruction = CloseAccount::new(LIGHT_TOKEN_PROGRAM_ID, account, destination, owner)
        .instruction()
        .expect("Failed to create instruction");

    // Hardcoded expected instruction
    let expected = Instruction {
        program_id: LIGHT_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(account, false), // account: writable, not signer
            AccountMeta::new(destination, false), // destination: writable, not signer
            AccountMeta::new(owner, true),    // owner: writable, signer
            AccountMeta::new(LIGHT_TOKEN_RENT_SPONSOR, false), // rent_sponsor: writable, not signer
        ],
        data: vec![9u8], // CloseAccount discriminator
    };

    assert_eq!(
        instruction, expected,
        "CloseAccount instruction should match expected"
    );
}

/// Test CloseAccount instruction with custom rent sponsor.
/// Verifies the rent_sponsor account is replaced with the custom one.
#[test]
fn test_close_account_custom_rent_sponsor() {
    // Use deterministic pubkeys for regression testing
    let account = Pubkey::new_from_array([1u8; 32]);
    let destination = Pubkey::new_from_array([2u8; 32]);
    let owner = Pubkey::new_from_array([3u8; 32]);
    let custom_sponsor = Pubkey::new_from_array([4u8; 32]);

    let instruction = CloseAccount::new(LIGHT_TOKEN_PROGRAM_ID, account, destination, owner)
        .custom_rent_sponsor(custom_sponsor)
        .instruction()
        .expect("Failed to create instruction");

    // Hardcoded expected instruction with custom rent sponsor
    let expected = Instruction {
        program_id: LIGHT_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(account, false), // account: writable, not signer
            AccountMeta::new(destination, false), // destination: writable, not signer
            AccountMeta::new(owner, true),    // owner: writable, signer
            AccountMeta::new(custom_sponsor, false), // custom_sponsor: writable, not signer
        ],
        data: vec![9u8], // CloseAccount discriminator
    };

    assert_eq!(
        instruction, expected,
        "CloseAccount instruction with custom rent sponsor should match expected"
    );
}
