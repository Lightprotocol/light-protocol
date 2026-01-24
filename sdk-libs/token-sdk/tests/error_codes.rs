//! Tests for error code stability.
//!
//! These tests ensure that error codes remain stable across versions.
//! Changing error codes would break client-side error handling.

use std::collections::HashSet;

use light_token::error::LightTokenError;

#[test]
fn test_error_codes_start_at_17500() {
    let first_error: u32 = LightTokenError::SplInterfaceRequired.into();
    assert_eq!(
        first_error, 17500,
        "First error code must be 17500 to avoid conflicts with TokenSdkError"
    );
}

#[test]
fn test_error_codes_unique() {
    let codes: Vec<u32> = vec![
        LightTokenError::SplInterfaceRequired.into(),
        LightTokenError::IncompleteSplInterface.into(),
        LightTokenError::UseRegularSplTransfer.into(),
        LightTokenError::CannotDetermineAccountType.into(),
        LightTokenError::MissingMintAccount.into(),
        LightTokenError::MissingSplTokenProgram.into(),
        LightTokenError::MissingSplInterfacePda.into(),
        LightTokenError::MissingSplInterfacePdaBump.into(),
        LightTokenError::SplTokenProgramMismatch.into(),
        LightTokenError::InvalidAccountData.into(),
        LightTokenError::SerializationError.into(),
    ];

    let unique_codes: HashSet<u32> = codes.iter().copied().collect();

    assert_eq!(
        codes.len(),
        unique_codes.len(),
        "All error codes must be unique"
    );
}

#[test]
fn test_spl_interface_required_is_17500() {
    let code: u32 = LightTokenError::SplInterfaceRequired.into();
    assert_eq!(code, 17500, "SplInterfaceRequired must be 17500");
}

#[test]
fn test_error_display_messages() {
    // Test each error's display message is non-empty
    assert!(
        !LightTokenError::SplInterfaceRequired.to_string().is_empty(),
        "SplInterfaceRequired must have a non-empty display message"
    );
    assert!(
        !LightTokenError::IncompleteSplInterface
            .to_string()
            .is_empty(),
        "IncompleteSplInterface must have a non-empty display message"
    );
    assert!(
        !LightTokenError::UseRegularSplTransfer
            .to_string()
            .is_empty(),
        "UseRegularSplTransfer must have a non-empty display message"
    );
    assert!(
        !LightTokenError::CannotDetermineAccountType
            .to_string()
            .is_empty(),
        "CannotDetermineAccountType must have a non-empty display message"
    );
    assert!(
        !LightTokenError::MissingMintAccount.to_string().is_empty(),
        "MissingMintAccount must have a non-empty display message"
    );
    assert!(
        !LightTokenError::MissingSplTokenProgram
            .to_string()
            .is_empty(),
        "MissingSplTokenProgram must have a non-empty display message"
    );
    assert!(
        !LightTokenError::MissingSplInterfacePda
            .to_string()
            .is_empty(),
        "MissingSplInterfacePda must have a non-empty display message"
    );
    assert!(
        !LightTokenError::MissingSplInterfacePdaBump
            .to_string()
            .is_empty(),
        "MissingSplInterfacePdaBump must have a non-empty display message"
    );
    assert!(
        !LightTokenError::SplTokenProgramMismatch
            .to_string()
            .is_empty(),
        "SplTokenProgramMismatch must have a non-empty display message"
    );
    assert!(
        !LightTokenError::InvalidAccountData.to_string().is_empty(),
        "InvalidAccountData must have a non-empty display message"
    );
    assert!(
        !LightTokenError::SerializationError.to_string().is_empty(),
        "SerializationError must have a non-empty display message"
    );
}
