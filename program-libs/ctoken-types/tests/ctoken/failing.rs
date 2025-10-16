use light_ctoken_types::{
    error::CTokenError,
    state::{CToken, CompressedTokenConfig},
};
use light_zero_copy::ZeroCopyNew;

#[test]
fn test_compressed_token_new_zero_copy_buffer_too_small() {
    let config = CompressedTokenConfig {
        delegate: false,
        is_native: false,
        close_authority: false,
        extensions: vec![],
    };

    // Create buffer that's too small
    let mut buffer = vec![0u8; 100]; // Less than 165 bytes required
    let result = CToken::new_zero_copy(&mut buffer, config);

    // Should fail with size error
    assert!(result.is_err());
}

#[test]
fn test_zero_copy_at_checked_uninitialized_account() {
    // Create a 165-byte buffer with all zeros (byte 108 = 0, uninitialized)
    let buffer = vec![0u8; 165];

    // This should fail because byte 108 is 0 (not initialized)
    let result = CToken::zero_copy_at_checked(&buffer);

    // Assert it returns InvalidAccountState error
    assert!(matches!(result, Err(CTokenError::InvalidAccountState)));
}

#[test]
fn test_zero_copy_at_mut_checked_uninitialized_account() {
    // Create a 165-byte mutable buffer with all zeros
    let mut buffer = vec![0u8; 165];

    // This should fail because byte 108 is 0 (not initialized)
    let result = CToken::zero_copy_at_mut_checked(&mut buffer);

    // Assert it returns InvalidAccountState error
    assert!(matches!(result, Err(CTokenError::InvalidAccountState)));
}

#[test]
fn test_zero_copy_at_checked_frozen_account() {
    // Create a 165-byte buffer with byte 108 = 2 (AccountState::Frozen)
    let mut buffer = vec![0u8; 165];
    buffer[108] = 2; // AccountState::Frozen

    // This should fail because byte 108 is 2 (frozen, not initialized)
    let result = CToken::zero_copy_at_checked(&buffer);

    // Assert it returns InvalidAccountState error
    assert!(matches!(result, Err(CTokenError::InvalidAccountState)));
}

#[test]
fn test_zero_copy_at_mut_checked_frozen_account() {
    // Create a 165-byte mutable buffer with byte 108 = 2
    let mut buffer = vec![0u8; 165];
    buffer[108] = 2; // AccountState::Frozen

    // This should fail because byte 108 is 2 (frozen, not initialized)
    let result = CToken::zero_copy_at_mut_checked(&mut buffer);

    // Assert it returns InvalidAccountState error
    assert!(matches!(result, Err(CTokenError::InvalidAccountState)));
}

#[test]
fn test_zero_copy_at_checked_buffer_too_small() {
    // Create a 100-byte buffer (less than 109 bytes minimum)
    let buffer = vec![0u8; 100];

    // This should fail because buffer is too small
    let result = CToken::zero_copy_at_checked(&buffer);

    // Assert it returns InvalidAccountData error
    assert!(matches!(result, Err(CTokenError::InvalidAccountData)));
}

#[test]
fn test_zero_copy_at_mut_checked_buffer_too_small() {
    // Create a 100-byte mutable buffer
    let mut buffer = vec![0u8; 100];

    // This should fail because buffer is too small
    let result = CToken::zero_copy_at_mut_checked(&mut buffer);

    // Assert it returns InvalidAccountData error
    assert!(matches!(result, Err(CTokenError::InvalidAccountData)));
}
