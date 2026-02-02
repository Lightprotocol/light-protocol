//! Tests for LightDiscriminator and AnchorDiscriminator derive macros.
//!
//! Verifies that both discriminator formats produce expected values
//! and that they differ from each other.

use light_sdk::{AnchorDiscriminator, LightDiscriminator};

/// Struct using Light discriminator format (SHA256("{name}")[0..8])
#[derive(LightDiscriminator)]
pub struct LightFormatAccount;

/// Struct using Anchor discriminator format (SHA256("account:{name}")[0..8])
#[derive(AnchorDiscriminator)]
pub struct AnchorFormatAccount;

/// Struct for testing both formats produce different values
#[derive(LightDiscriminator)]
pub struct TestAccount;

/// Same name but with Anchor format to compare
#[derive(AnchorDiscriminator)]
pub struct TestAccountAnchor;

#[test]
fn test_light_discriminator_format() {
    // SHA256("LightFormatAccount")[0..8] = f9 30 5f 8c 86 2d 21 c3
    const EXPECTED: [u8; 8] = [249, 48, 95, 140, 134, 45, 33, 195];
    assert_eq!(
        LightFormatAccount::LIGHT_DISCRIMINATOR,
        EXPECTED,
        "LightDiscriminator should use SHA256(name) format"
    );
}

#[test]
fn test_anchor_discriminator_format() {
    // SHA256("account:AnchorFormatAccount")[0..8] = f2 3b 7f 36 38 66 b8 c7
    const EXPECTED: [u8; 8] = [242, 59, 127, 54, 56, 102, 184, 199];
    assert_eq!(
        AnchorFormatAccount::LIGHT_DISCRIMINATOR,
        EXPECTED,
        "AnchorDiscriminator should use SHA256(account:name) format"
    );
}

#[test]
fn test_discriminators_are_different() {
    // Light format: SHA256("TestAccount")[0..8]
    let light_discriminator = TestAccount::LIGHT_DISCRIMINATOR;

    // Anchor format: SHA256("account:TestAccountAnchor")[0..8]
    // Note: We can't derive both on the same struct, so we use a different struct name
    // The key is that even if we manually computed SHA256("account:TestAccount"),
    // it would differ from SHA256("TestAccount")
    let anchor_discriminator = TestAccountAnchor::LIGHT_DISCRIMINATOR;

    // Verify they're different (even though they have similar names)
    assert_ne!(
        light_discriminator, anchor_discriminator,
        "Light and Anchor discriminators should produce different values"
    );
}

#[test]
fn test_discriminator_trait_methods() {
    // Test that the discriminator() method returns the same value as the constant
    assert_eq!(
        LightFormatAccount::discriminator(),
        LightFormatAccount::LIGHT_DISCRIMINATOR,
        "discriminator() method should return LIGHT_DISCRIMINATOR constant"
    );

    assert_eq!(
        AnchorFormatAccount::discriminator(),
        AnchorFormatAccount::LIGHT_DISCRIMINATOR,
        "discriminator() method should return LIGHT_DISCRIMINATOR constant"
    );
}

#[test]
fn test_discriminator_slice() {
    // Test that LIGHT_DISCRIMINATOR_SLICE matches LIGHT_DISCRIMINATOR
    assert_eq!(
        LightFormatAccount::LIGHT_DISCRIMINATOR_SLICE,
        &LightFormatAccount::LIGHT_DISCRIMINATOR,
        "LIGHT_DISCRIMINATOR_SLICE should be a slice of LIGHT_DISCRIMINATOR"
    );

    assert_eq!(
        AnchorFormatAccount::LIGHT_DISCRIMINATOR_SLICE,
        &AnchorFormatAccount::LIGHT_DISCRIMINATOR,
        "LIGHT_DISCRIMINATOR_SLICE should be a slice of LIGHT_DISCRIMINATOR"
    );
}
