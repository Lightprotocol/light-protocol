//! Tests for TransferFeeConfig extension behavior during compress/decompress.
//!
//! This module tests the compress_only behavior with only the TransferFeeConfig extension.

use serial_test::serial;
use spl_token_2022::extension::ExtensionType;

use super::shared::{run_compress_and_close_extension_test, CompressAndCloseTestConfig};

/// Test compress -> decompress cycle with only TransferFeeConfig extension.
#[tokio::test]
#[serial]
async fn test_transfer_fee_only() {
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        extensions: &[ExtensionType::TransferFeeConfig],
        delegate_config: None,
        is_frozen: false,
        use_permanent_delegate_for_decompress: false,
        use_delegate_for_decompress: false,
    })
    .await
    .unwrap();
}
