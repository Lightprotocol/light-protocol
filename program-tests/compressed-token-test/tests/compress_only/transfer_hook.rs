//! Tests for TransferHook extension behavior during compress/decompress.
//!
//! This module tests the compress_only behavior with only the TransferHook extension.

use serial_test::serial;
use spl_token_2022::extension::ExtensionType;

use super::shared::{run_compress_and_close_extension_test, CompressAndCloseTestConfig};

/// Test compress -> decompress cycle with only TransferHook extension.
#[tokio::test]
#[serial]
async fn test_transfer_hook_only() {
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        extensions: &[ExtensionType::TransferHook],
        delegate_config: None,
        is_frozen: false,
        use_permanent_delegate_for_decompress: false,
        use_delegate_for_decompress: false,
    })
    .await
    .unwrap();
}
