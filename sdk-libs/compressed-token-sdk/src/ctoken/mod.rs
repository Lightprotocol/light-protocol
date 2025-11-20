mod close;
mod compressible;
mod create;
mod create_ata;
mod create_cmint;
mod mint_to;
mod transfer_ctoken;
mod transfer_interface;

pub use close::*;
pub use compressible::{CompressibleParams, CompressibleParamsInfos};
pub use create::*;
pub use create_ata::*;
pub use create_cmint::*;
use light_compressed_token_types::POOL_SEED;
use light_compressible::config::CompressibleConfig;
pub use light_ctoken_types::{
    instructions::extensions::{compressible::CompressToPubkey, ExtensionInstructionData},
    state::TokenDataVersion,
};
pub use mint_to::*;
use solana_pubkey::{pubkey, Pubkey};
pub use transfer_ctoken::*;
pub use transfer_interface::*;

pub const CTOKEN_PROGRAM_ID: Pubkey = pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

pub const CTOKEN_CPI_AUTHORITY: Pubkey = pubkey!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy");

pub const COMPRESSIBLE_CONFIG_V1: Pubkey = pubkey!("ACXg8a7VaqecBWrSbdu73W4Pg9gsqXJ3EXAqkHyhvVXg");

pub const RENT_SPONSOR: Pubkey = pubkey!("r18WwUxfG8kQ69bQPAB2jV6zGNKy3GosFGctjQoV4ti");

/// Returns the program ID for the Compressed Token Program
pub fn id() -> Pubkey {
    CTOKEN_PROGRAM_ID
}

/// Return the cpi authority pda of the Compressed Token Program.
pub fn cpi_authority() -> Pubkey {
    CTOKEN_CPI_AUTHORITY
}

pub fn get_token_pool_address_and_bump(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[POOL_SEED, mint.as_ref()], &CTOKEN_PROGRAM_ID)
}

/// Returns the associated ctoken address for a given owner and mint.
pub fn get_associated_ctoken_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    get_associated_ctoken_address_and_bump(owner, mint).0
}

/// Returns the associated ctoken address and bump for a given owner and mint.
pub fn get_associated_ctoken_address_and_bump(owner: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[&owner.to_bytes(), &id().to_bytes(), &mint.to_bytes()],
        &id(),
    )
}

pub fn config_pda() -> Pubkey {
    COMPRESSIBLE_CONFIG_V1
}

pub fn rent_sponsor_pda() -> Pubkey {
    RENT_SPONSOR
}

pub fn compression_authority_pda() -> Pubkey {
    CompressibleConfig::ctoken_v1_compression_authority_pda()
}
