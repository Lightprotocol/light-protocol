//! Tests for delegate-related behavior during compress/decompress.
//!
//! This module tests:
//! - Delegated amount preservation through compress -> decompress cycle
//! - Regular delegate decompression authorization

use serial_test::serial;
use solana_sdk::signature::Keypair;

use super::shared::{
    run_compress_and_close_extension_test, CompressAndCloseTestConfig, ALL_EXTENSIONS,
};

/// Test that delegated amount is preserved through compress -> decompress cycle.
#[tokio::test]
#[serial]
async fn test_compress_and_close_with_delegated_amount() {
    let delegate = Keypair::new();
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        extensions: ALL_EXTENSIONS,
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
        extensions: ALL_EXTENSIONS,
        delegate_config: Some((delegate, 500_000_000)),
        is_frozen: false,
        use_permanent_delegate_for_decompress: false,
        use_delegate_for_decompress: true,
    })
    .await
    .unwrap();
}

/// Test delegated amount with no extensions.
#[tokio::test]
#[serial]
async fn test_compress_and_close_with_delegated_amount_no_extensions() {
    let delegate = Keypair::new();
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        extensions: &[],
        delegate_config: Some((delegate, 500_000_000)),
        is_frozen: false,
        use_permanent_delegate_for_decompress: false,
        use_delegate_for_decompress: false,
    })
    .await
    .unwrap();
}

/// Test delegate decompress with no extensions.
#[tokio::test]
#[serial]
async fn test_compress_and_close_delegate_decompress_no_extensions() {
    let delegate = Keypair::new();
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        extensions: &[],
        delegate_config: Some((delegate, 500_000_000)),
        is_frozen: false,
        use_permanent_delegate_for_decompress: false,
        use_delegate_for_decompress: true,
    })
    .await
    .unwrap();
}

/// Test that orphan delegate (delegate set, delegated_amount = 0) is preserved
/// through compress -> decompress cycle.
///
/// Covers spec requirements:
/// - #12: Orphan delegate (delegate set, delegated_amount = 0)
/// - #17: Restores orphan delegate on decompress
/// - #26: Full round-trip orphan delegate state preserved
#[tokio::test]
#[serial]
async fn test_compress_and_close_preserves_orphan_delegate() {
    let delegate = Keypair::new();
    // delegate_config with delegated_amount = 0 creates an orphan delegate
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        extensions: ALL_EXTENSIONS,
        delegate_config: Some((delegate, 0)), // delegated_amount = 0 but delegate is set
        is_frozen: false,
        use_permanent_delegate_for_decompress: false,
        use_delegate_for_decompress: false,
    })
    .await
    .unwrap();
}

/// Test orphan delegate with no extensions.
#[tokio::test]
#[serial]
async fn test_compress_and_close_orphan_delegate_no_extensions() {
    let delegate = Keypair::new();
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        extensions: &[],
        delegate_config: Some((delegate, 0)),
        is_frozen: false,
        use_permanent_delegate_for_decompress: false,
        use_delegate_for_decompress: false,
    })
    .await
    .unwrap();
}

/// Test that orphan delegate can still decompress (delegate has authority even with 0 amount).
#[tokio::test]
#[serial]
async fn test_orphan_delegate_can_decompress() {
    let delegate = Keypair::new();
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        extensions: ALL_EXTENSIONS,
        delegate_config: Some((delegate, 0)),
        is_frozen: false,
        use_permanent_delegate_for_decompress: false,
        use_delegate_for_decompress: true, // delegate signs for decompress
    })
    .await
    .unwrap();
}
