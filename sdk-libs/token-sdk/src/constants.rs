//! Constants for Light Token SDK.
//!
//! Re-exports constants from `light_compressed_token_sdk::constants` and adds
//! Light Token specific constants.

// Re-export all constants from compressed-token-sdk
pub use light_compressed_token_sdk::constants::*;
use light_compressible::config::CompressibleConfig;
use solana_pubkey::{pubkey, Pubkey};

/// CPI Authority PDA for the Compressed Token Program
pub const LIGHT_TOKEN_CPI_AUTHORITY: Pubkey =
    pubkey!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy");

/// Returns the program ID for the Compressed Token Program
pub fn id() -> Pubkey {
    LIGHT_TOKEN_PROGRAM_ID
}

/// Return the cpi authority pda of the Compressed Token Program.
pub fn cpi_authority() -> Pubkey {
    LIGHT_TOKEN_CPI_AUTHORITY
}

/// Default compressible config PDA (V1)
pub const LIGHT_TOKEN_CONFIG: Pubkey = pubkey!("ACXg8a7VaqecBWrSbdu73W4Pg9gsqXJ3EXAqkHyhvVXg");

/// Default rent sponsor PDA (V1)
pub const RENT_SPONSOR_V1: Pubkey = pubkey!("r18WwUxfG8kQ69bQPAB2jV6zGNKy3GosFGctjQoV4ti");

/// Returns the default compressible config PDA.
pub fn config_pda() -> Pubkey {
    CompressibleConfig::light_token_v1_config_pda()
}

/// Returns the default rent sponsor PDA.
pub fn rent_sponsor_pda() -> Pubkey {
    CompressibleConfig::light_token_v1_rent_sponsor_pda()
}

/// Returns the compression authority PDA.
pub fn compression_authority_pda() -> Pubkey {
    CompressibleConfig::light_token_v1_compression_authority_pda()
}
