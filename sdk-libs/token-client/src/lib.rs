pub mod actions;
pub mod instructions;

// Re-export the main utility functions for easy access
use solana_pubkey::{pubkey, Pubkey};
pub use utils::{
    derive_compressed_address, derive_compressed_address_from_mint_signer, find_mint_address,
};

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

    /// Derives the cToken program mint PDA from the provided signer pubkey (keypair or PDA).
    /// The signer must sign when creating the SPL mint PDA on-chain.
    pub fn find_mint_address(signer: Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[CTOKEN_MINT_SEED, signer.to_bytes().as_ref()], &ID)
    }

    pub fn derive_compressed_address(mint: Pubkey, address_tree: &Pubkey) -> [u8; 32] {
        derive_address(&mint.to_bytes(), &address_tree.to_bytes(), &ID.to_bytes())
    }

    /// Comprehensive helper that derives all addresses from a signer in one call
    /// Returns: (mint_address, mint_bump, compressed_address)
    pub fn derive_compressed_address_from_mint_signer(
        signer: Pubkey,
        address_tree: &Pubkey,
    ) -> (Pubkey, u8, [u8; 32]) {
        let (mint_address, mint_bump) = find_mint_address(signer);
        let compressed_address = derive_compressed_address(mint_address, address_tree);
        (mint_address, mint_bump, compressed_address)
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

    /// Derives the SPL mint PDA from a signer keypair
    ///
    /// # Arguments
    /// * `signer` - The signer pubkey used as seed
    ///
    /// # Returns
    /// Tuple of (mint_pda, bump_seed)
    /// Derives the Compressed Token Program mint PDA from a signer pubkey.
    ///
    /// This derives the cToken program mint PDA for a given keypair or PDA; that signer must sign.
    pub fn find_mint_address(signer: &Pubkey) -> (Pubkey, u8) {
        sdk_find_mint_address(signer)
    }

    /// Derives the compressed address from a mint PDA and address tree
    ///
    /// # Arguments
    /// * `mint` - The mint PDA
    /// * `address_tree` - The address tree pubkey
    ///
    /// # Returns
    /// The compressed address as [u8; 32]
    pub fn derive_compressed_address(mint: &Pubkey, address_tree: &Pubkey) -> [u8; 32] {
        sdk_derive_address(
            &mint.to_bytes(),
            &address_tree.to_bytes(),
            &super::ctoken::ID.to_bytes(),
        )
    }

    /// Comprehensive helper that derives all addresses from a signer in one call
    ///
    /// This is the main function you should use for mint derivation.
    ///
    /// # Arguments
    /// * `signer` - The signer keypair pubkey
    /// * `address_tree` - The address tree pubkey
    ///
    /// # Returns
    /// Tuple of (mint_address, mint_bump, compressed_address)
    ///
    /// # Example
    /// ```rust
    /// use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
    /// use light_token_client::utils::derive_compressed_address_from_signer;
    ///
    /// let signer = Keypair::new();
    /// let address_tree = Pubkey::new_unique();
    /// let (mint_pda, mint_bump, compressed_address) =
    ///     derive_compressed_address_from_signer(&signer.pubkey(), &address_tree);
    ///
    /// println!("Mint PDA: {}", mint_pda);
    /// println!("Mint Bump: {}", mint_bump);
    /// println!("Compressed Address: {:?}", compressed_address);
    /// ```
    pub fn derive_compressed_address_from_mint_signer(
        signer: &Pubkey,
        address_tree: &Pubkey,
    ) -> (Pubkey, u8, [u8; 32]) {
        sdk_derive_compressed_address_from_mint_signer(signer, address_tree)
    }
}
