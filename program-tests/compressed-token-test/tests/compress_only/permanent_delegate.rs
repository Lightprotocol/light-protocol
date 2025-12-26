//! Tests for PermanentDelegate extension behavior during compress/decompress.
//!
//! This module tests that the permanent delegate can decompress
//! CompressedOnly tokens on behalf of the owner.

use serial_test::serial;

use super::shared::{run_compress_and_close_extension_test, CompressAndCloseTestConfig};

/// Test that permanent delegate can decompress CompressedOnly tokens.
#[tokio::test]
#[serial]
async fn test_compress_and_close_with_permanent_delegate() {
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        delegate_config: None,
        is_frozen: false,
        use_permanent_delegate_for_decompress: true,
        use_delegate_for_decompress: false,
    })
    .await
    .unwrap();
}
