use light_compressed_account::address::derive_address as sdk_derive_address;
use light_compressed_token_sdk::instructions::{
    derive_compressed_address_from_mint_signer as sdk_derive_compressed_address_from_mint_signer,
    find_mint_address as sdk_find_mint_address,
};
use solana_pubkey::Pubkey;

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
