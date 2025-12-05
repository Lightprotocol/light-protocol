use light_compressed_token_types::constants::POOL_SEED;
use light_ctoken_interface::COMPRESSED_TOKEN_PROGRAM_ID;
use solana_pubkey::Pubkey;

use crate::{AnchorDeserialize, AnchorSerialize};

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize, PartialEq)]
pub struct TokenPool {
    pub pubkey: Pubkey,
    pub bump: u8,
    pub index: u8,
}

/// Derive the token pool pda for a given mint
pub fn get_token_pool_pda(mint: &Pubkey) -> Pubkey {
    get_token_pool_pda_with_index(mint, 0)
}

/// Find the token pool pda for a given mint and index
pub fn find_token_pool_pda_with_index(mint: &Pubkey, token_pool_index: u8) -> (Pubkey, u8) {
    let seeds = &[POOL_SEED, mint.as_ref(), &[token_pool_index]];
    let seeds = if token_pool_index == 0 {
        &seeds[..2]
    } else {
        &seeds[..]
    };
    Pubkey::find_program_address(seeds, &Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID))
}

/// Get the token pool pda for a given mint and index
pub fn get_token_pool_pda_with_index(mint: &Pubkey, token_pool_index: u8) -> Pubkey {
    find_token_pool_pda_with_index(mint, token_pool_index).0
}

/// Derive token pool information for a given mint
pub fn derive_token_pool(mint: &solana_pubkey::Pubkey, index: u8) -> TokenPool {
    let (pubkey, bump) = find_token_pool_pda_with_index(mint, index);
    TokenPool {
        pubkey,
        bump,
        index,
    }
}
