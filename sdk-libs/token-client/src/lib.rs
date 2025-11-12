pub mod actions;
pub mod instructions;

// Re-export the main utility functions for easy access
use solana_pubkey::{pubkey, Pubkey};

pub const CTOKEN_PROGRAM_ID: Pubkey = pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

pub const CTOKEN_CPI_AUTHORITY: Pubkey = pubkey!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy");

pub mod ctoken {
    use light_compressed_token_sdk::POOL_SEED;
    use light_compressible::config::CompressibleConfig;
    use solana_pubkey::Pubkey;

    use super::{CTOKEN_CPI_AUTHORITY, CTOKEN_PROGRAM_ID};

    pub const ID: Pubkey = CTOKEN_PROGRAM_ID;

    /// Returns the program ID for the Compressed Token Program
    pub fn id() -> Pubkey {
        ID
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
        Pubkey::find_program_address(
            &[&owner.to_bytes(), &id().to_bytes(), &mint.to_bytes()],
            &id(),
        )
        .0
    }
    /// Returns the associated ctoken address and bump for a given owner and mint.
    pub fn get_associated_ctoken_address_and_bump(owner: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[&owner.to_bytes(), &id().to_bytes(), &mint.to_bytes()],
            &id(),
        )
    }

    pub use light_compressed_token_sdk::instructions::{
        create_compressed_mint::find_spl_mint_address, derive_cmint_from_spl_mint,
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
}
