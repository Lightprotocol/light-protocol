use light_ctoken_types::state::{CompressedToken, CompressedTokenConfig};
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
    let result = CompressedToken::new_zero_copy(&mut buffer, config);

    // Should fail with size error
    assert!(result.is_err());
}
