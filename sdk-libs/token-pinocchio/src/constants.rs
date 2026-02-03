//! Constants for Light Token Pinocchio SDK.
//!
//! Re-exports constants from `light_sdk_types::constants`.

// Re-export core constants
pub use light_sdk_types::constants::{
    ACCOUNT_COMPRESSION_AUTHORITY_PDA, ACCOUNT_COMPRESSION_PROGRAM_ID, CPI_AUTHORITY_PDA_SEED,
    LIGHT_SYSTEM_PROGRAM_ID, LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_PROGRAM_ID, LIGHT_TOKEN_RENT_SPONSOR,
    REGISTERED_PROGRAM_PDA,
};

/// CPI Authority PDA for the Light Token Program (as bytes)
pub const LIGHT_TOKEN_CPI_AUTHORITY: [u8; 32] =
    light_macros::pubkey_array!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy");

/// Returns the program ID for the Light Token Program as bytes
#[inline]
pub const fn id() -> [u8; 32] {
    LIGHT_TOKEN_PROGRAM_ID
}

/// Return the CPI authority PDA of the Light Token Program as bytes.
#[inline]
pub const fn cpi_authority() -> [u8; 32] {
    LIGHT_TOKEN_CPI_AUTHORITY
}

/// Returns the default compressible config PDA as bytes.
#[inline]
pub const fn config_pda() -> [u8; 32] {
    LIGHT_TOKEN_CONFIG
}

/// Returns the default rent sponsor PDA as bytes.
#[inline]
pub const fn rent_sponsor_pda() -> [u8; 32] {
    LIGHT_TOKEN_RENT_SPONSOR
}
