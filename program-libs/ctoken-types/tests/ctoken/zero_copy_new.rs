//! Contains functional zero copy tests for:
//! - ZeroCopyNew
//!
//! Tests:
//! 1.test_compressed_token_new_zero_copy
//! 2. test_compressed_token_new_zero_copy_with_delegate
//! 3. test_compressed_token_new_zero_copy_all_options

use light_ctoken_types::state::ctoken::{CToken, CompressedTokenConfig};
use light_zero_copy::traits::ZeroCopyNew;

#[test]
fn test_compressed_token_new_zero_copy() {
    let config = CompressedTokenConfig {
        delegate: false,
        is_native: false,
        close_authority: false,
        extensions: vec![],
    };

    // Calculate required buffer size
    let required_size = CToken::byte_len(&config).unwrap();
    assert_eq!(required_size, 165); // SPL Token account size

    // Create buffer and initialize
    let mut buffer = vec![0u8; required_size];
    let (compressed_token, remaining_bytes) =
        CToken::new_zero_copy(&mut buffer, config).expect("Failed to initialize compressed token");

    // Verify the remaining bytes length
    assert_eq!(remaining_bytes.len(), 0);
    // Verify the zero-copy structure reflects the discriminators
    assert!(compressed_token.delegate.is_none());
    assert!(compressed_token.is_native.is_none());
    assert!(compressed_token.close_authority.is_none());
    assert!(compressed_token.extensions.is_none());
    // Verify the discriminator bytes are set correctly
    assert_eq!(buffer[72], 0); // delegate discriminator should be 0 (None)
    assert_eq!(buffer[109], 0); // is_native discriminator should be 0 (None)
    assert_eq!(buffer[129], 0); // close_authority discriminator should be 0 (None)
}

#[test]
fn test_compressed_token_new_zero_copy_with_delegate() {
    let config = CompressedTokenConfig {
        delegate: true,
        is_native: false,
        close_authority: false,
        extensions: vec![],
    };

    // Create buffer and initialize
    let mut buffer = vec![0u8; CToken::byte_len(&config).unwrap()];
    let (compressed_token, _) = CToken::new_zero_copy(&mut buffer, config)
        .expect("Failed to initialize compressed token with delegate");
    // The delegate field should be Some (though the pubkey will be zero)
    assert!(compressed_token.delegate.is_some());
    assert!(compressed_token.is_native.is_none());
    assert!(compressed_token.close_authority.is_none());
    // Verify delegate discriminator is set to 1 (Some)
    assert_eq!(buffer[72], 1); // delegate discriminator should be 1 (Some)
    assert_eq!(buffer[109], 0); // is_native discriminator should be 0 (None)
    assert_eq!(buffer[129], 0); // close_authority discriminator should be 0 (None)
}
#[test]
fn test_compressed_token_new_zero_copy_with_is_native() {
    let config = CompressedTokenConfig {
        delegate: false,
        is_native: true,
        close_authority: false,
        extensions: vec![],
    };

    // Create buffer and initialize
    let mut buffer = vec![0u8; CToken::byte_len(&config).unwrap()];
    let (compressed_token, _) = CToken::new_zero_copy(&mut buffer, config)
        .expect("Failed to initialize compressed token with is_native");

    // The is_native field should be Some (though the value will be zero)
    assert!(compressed_token.delegate.is_none());
    assert!(compressed_token.is_native.is_some());
    assert!(compressed_token.close_authority.is_none());

    // Verify is_native discriminator is set to 1 (Some)
    assert_eq!(buffer[72], 0); // delegate discriminator should be 0 (None)
    assert_eq!(buffer[109], 1); // is_native discriminator should be 1 (Some)
    assert_eq!(buffer[129], 0); // close_authority discriminator should be 0 (None)
}
#[test]
fn test_compressed_token_new_zero_copy_all_options() {
    let config = CompressedTokenConfig {
        delegate: true,
        is_native: true,
        close_authority: true,
        extensions: vec![],
    };

    // Create buffer and initialize
    let mut buffer = vec![0u8; CToken::byte_len(&config).unwrap()];
    let (compressed_token, _) = CToken::new_zero_copy(&mut buffer, config)
        .expect("Failed to initialize compressed token with all options");

    // All optional fields should be Some
    assert!(compressed_token.delegate.is_some());
    assert!(compressed_token.is_native.is_some());
    assert!(compressed_token.close_authority.is_some());
    // Verify all discriminators are set to 1 (Some)
    assert_eq!(buffer[72], 1); // delegate discriminator should be 1 (Some)
    assert_eq!(buffer[109], 1); // is_native discriminator should be 1 (Some)
    assert_eq!(buffer[129], 1); // close_authority discriminator should be 1 (Some)
}
