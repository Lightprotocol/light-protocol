pub mod actions;
pub mod instructions;
use solana_pubkey::{pubkey, Pubkey};

pub const CTOKEN_PROGRAM_ID: Pubkey = pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

pub const CTOKEN_CPI_AUTHORITY: Pubkey = pubkey!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy");

pub mod ctoken {
    use light_compressed_account::address::derive_address;
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

    pub const CTOKEN_MINT_SEED: &[u8] = &[
        //  b"compressed_mint"
        99, 111, 109, 112, 114, 101, 115, 115, 101, 100, 95, 109, 105, 110, 116,
    ];

    pub fn find_mint_address(mint_signer: Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[CTOKEN_MINT_SEED, mint_signer.to_bytes().as_ref()], &ID)
    }

    pub fn derive_ctoken_mint_address(mint_address: Pubkey, address_tree: &Pubkey) -> [u8; 32] {
        derive_address(
            &mint_address.to_bytes(),
            &address_tree.to_bytes(),
            &ID.to_bytes(),
        )
    }

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

    pub fn derive_ctoken_rent_sponsor(_version: Option<u64>) -> (Pubkey, u8) {
        // Derive the rent_recipient PDA
        // let version = version.unwrap_or(1);
        let version = 1u16;
        Pubkey::find_program_address(
            &[
                b"rent_sponsor".as_slice(),
                (version as u16).to_le_bytes().as_slice(),
            ],
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
