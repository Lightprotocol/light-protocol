//! Tests for delegate-related behavior during compress/decompress.
//!
//! This module tests:
//! - Delegated amount preservation through compress -> decompress cycle
//! - Regular delegate decompression authorization

use serial_test::serial;
use solana_sdk::signature::Keypair;

use super::shared::{run_compress_and_close_extension_test, CompressAndCloseTestConfig};

/// Test that delegated amount is preserved through compress -> decompress cycle.
#[tokio::test]
#[serial]
async fn test_compress_and_close_with_delegated_amount() {
    let delegate = Keypair::new();
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        delegate_config: Some((delegate, 500_000_000)),
        is_frozen: false,
        use_permanent_delegate_for_decompress: false,
        use_delegate_for_decompress: false,
    })
    .await
    .unwrap();
}

/// Test that regular delegate can decompress CompressedOnly tokens.
#[tokio::test]
#[serial]
async fn test_compress_and_close_delegate_decompress() {
    let delegate = Keypair::new();
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        delegate_config: Some((delegate, 500_000_000)),
        is_frozen: false,
        use_permanent_delegate_for_decompress: false,
        use_delegate_for_decompress: true,
    })
    .await
    .unwrap();
}
