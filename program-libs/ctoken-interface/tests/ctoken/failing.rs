use light_compressed_account::Pubkey;
use light_ctoken_interface::{
    error::CTokenError,
    state::{CToken, CompressedTokenConfig, BASE_TOKEN_ACCOUNT_SIZE},
};
use light_zero_copy::ZeroCopyNew;

fn default_config() -> CompressedTokenConfig {
    CompressedTokenConfig {
        mint: Pubkey::default(),
        owner: Pubkey::default(),
        state: 1,
        compression_only: false,
        extensions: None,
    }
}

#[test]
fn test_compressed_token_new_zero_copy_buffer_too_small() {
    let config = default_config();

    // Create buffer that's too small
    let mut buffer = vec![0u8; 100]; // Less than BASE_TOKEN_ACCOUNT_SIZE
    let result = CToken::new_zero_copy(&mut buffer, config);

    // Should fail with size error
    assert!(result.is_err());
}

#[test]
fn test_zero_copy_at_checked_uninitialized_account() {
    // Create a buffer with all zeros (state byte = 0, uninitialized)
    let buffer = vec![0u8; BASE_TOKEN_ACCOUNT_SIZE as usize];

    // This should fail because state byte is 0 (not initialized)
    let result = CToken::zero_copy_at_checked(&buffer);

    // Assert it returns InvalidAccountState error
    assert!(matches!(result, Err(CTokenError::InvalidAccountState)));
}

#[test]
fn test_zero_copy_at_mut_checked_uninitialized_account() {
    // Create a mutable buffer with all zeros
    let mut buffer = vec![0u8; BASE_TOKEN_ACCOUNT_SIZE as usize];

    // This should fail because state byte is 0 (not initialized)
    let result = CToken::zero_copy_at_mut_checked(&mut buffer);

    // Assert it returns InvalidAccountState error
    assert!(matches!(result, Err(CTokenError::InvalidAccountState)));
}

#[test]
fn test_zero_copy_at_checked_buffer_too_small() {
    // Create a 100-byte buffer (less than BASE_TOKEN_ACCOUNT_SIZE)
    let buffer = vec![0u8; 100];

    // This should fail because buffer is too small
    let result = CToken::zero_copy_at_checked(&buffer);

    // Assert it returns ZeroCopyError (buffer too small fails at zero_copy_at before checked validation)
    assert!(matches!(result, Err(CTokenError::ZeroCopyError(_))));
}

#[test]
fn test_zero_copy_at_mut_checked_buffer_too_small() {
    // Create a 100-byte mutable buffer
    let mut buffer = vec![0u8; 100];

    // This should fail because buffer is too small
    let result = CToken::zero_copy_at_mut_checked(&mut buffer);

    // Assert it returns ZeroCopyError (buffer too small fails at zero_copy_at_mut before checked validation)
    assert!(matches!(result, Err(CTokenError::ZeroCopyError(_))));
}
