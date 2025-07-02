pub mod actions;
pub mod instructions;

use solana_pubkey::{pubkey, Pubkey};

pub const COMPRESSED_TOKEN_PROGRAM_ID: Pubkey =
    pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

pub const COMPRESSED_TOKEN_CPI_AUTHORITY: Pubkey =
    pubkey!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy");

pub mod compressed_token {
    use light_compressed_account::address::derive_address;
    use light_compressed_token_sdk::POOL_SEED;
    use solana_pubkey::Pubkey;

    use super::{COMPRESSED_TOKEN_CPI_AUTHORITY, COMPRESSED_TOKEN_PROGRAM_ID};

    pub const ID: Pubkey = COMPRESSED_TOKEN_PROGRAM_ID;

    /// Returns the program ID for the Compressed Token Program
    pub fn id() -> Pubkey {
        ID
    }
    /// Return the cpi authority pda of the Compressed Token Program.
    pub fn cpi_authority() -> Pubkey {
        COMPRESSED_TOKEN_CPI_AUTHORITY
    }

    pub fn get_token_pool_address_and_bump(mint: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[POOL_SEED, mint.as_ref()], &COMPRESSED_TOKEN_PROGRAM_ID)
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

    pub const COMPRESSED_MINT_SEED: &[u8] = &[
        //  b"compressed_mint"
        99, 111, 109, 112, 114, 101, 115, 115, 101, 100, 95, 109, 105, 110, 116,
    ];

    pub fn find_mint_address(mint_signer: Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[COMPRESSED_MINT_SEED, &mint_signer.to_bytes().as_ref()],
            &ID,
        )
    }

    pub fn derive_compressed_mint_address(mint_address: Pubkey, address_tree: &Pubkey) -> [u8; 32] {
        derive_address(
            &mint_address.to_bytes(),
            &address_tree.to_bytes(),
            &ID.to_bytes(),
        )
    }
}
