//! Tests for frozen state preservation during compress/decompress.
//!
//! This module tests that frozen state is preserved when compressing
//! and decompressing CToken accounts with Token-2022 extensions.

use serial_test::serial;

use super::shared::{
    run_compress_and_close_extension_test, CompressAndCloseTestConfig, ALL_EXTENSIONS,
};

/// Test that frozen state is preserved through compress -> decompress cycle.
#[tokio::test]
#[serial]
async fn test_compress_and_close_frozen() {
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        extensions: ALL_EXTENSIONS,
        delegate_config: None,
        is_frozen: true,
        use_permanent_delegate_for_decompress: false,
        use_delegate_for_decompress: false,
    })
    .await
    .unwrap();
}

/// Test frozen state with no extensions.
#[tokio::test]
#[serial]
async fn test_compress_and_close_frozen_no_extensions() {
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        extensions: &[],
        delegate_config: None,
        is_frozen: true,
        use_permanent_delegate_for_decompress: false,
        use_delegate_for_decompress: false,
    })
    .await
    .unwrap();
}
