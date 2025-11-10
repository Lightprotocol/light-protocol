pub mod actions;
pub mod instructions;

// Re-export the main utility functions for easy access
use solana_pubkey::{pubkey, Pubkey};

pub const CTOKEN_PROGRAM_ID: Pubkey = pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

pub const CTOKEN_CPI_AUTHORITY: Pubkey = pubkey!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy");

pub mod ctoken {
    use light_compressed_token_sdk::POOL_SEED;
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

    pub fn derive_ctoken_program_config(_version: Option<u64>) -> (Pubkey, u8) {
        let version = 1u16;
        let registry_program_id =
            solana_pubkey::pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX");
        let (compressible_config_pda, config_bump) = Pubkey::find_program_address(
            &[b"compressible_config", &version.to_le_bytes()],
            &registry_program_id,
        );
        println!("compressible_config: {:?}", compressible_config_pda);
        (compressible_config_pda, config_bump)
    }

    // TODO: add version.
    pub fn derive_ctoken_rent_sponsor(_version: Option<u64>) -> (Pubkey, u8) {
        let version = 1u16;
        Pubkey::find_program_address(
            &[b"rent_sponsor".as_slice(), version.to_le_bytes().as_slice()],
            &solana_pubkey::pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m"),
        )
    }

    pub fn derive_ctoken_compression_authority(version: Option<u64>) -> (Pubkey, u8) {
        let registry_program_id =
            solana_pubkey::pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX");
        let (compression_authority, compression_authority_bump) = Pubkey::find_program_address(
            &[
                b"compression_authority".as_slice(),
                version.unwrap_or(1).to_le_bytes().as_slice(),
            ],
            &registry_program_id,
        );
        (compression_authority, compression_authority_bump)
    }
}
