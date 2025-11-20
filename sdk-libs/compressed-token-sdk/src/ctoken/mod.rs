pub mod close;
pub mod create_associated_token_account;
pub mod create_token_account;
pub mod transfer_ctoken;
pub mod transfer_interface;

use light_compressed_token_types::POOL_SEED;
use light_compressible::config::CompressibleConfig;
use solana_pubkey::{pubkey, Pubkey};

pub const CTOKEN_PROGRAM_ID: Pubkey = pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

pub const CTOKEN_CPI_AUTHORITY: Pubkey = pubkey!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy");

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

pub use crate::compressed_token::create_compressed_mint::{
    derive_cmint_from_spl_mint, find_spl_mint_address,
};

pub fn config_pda() -> Pubkey {
    CompressibleConfig::ctoken_v1_config_pda()
}

pub fn rent_sponsor_pda() -> Pubkey {
    CompressibleConfig::ctoken_v1_rent_sponsor_pda()
}

pub fn compression_authority_pda() -> Pubkey {
    CompressibleConfig::ctoken_v1_compression_authority_pda()
}
