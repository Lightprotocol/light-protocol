//! SPL interface PDA derivation utilities.

use light_compressed_token_types::constants::POOL_SEED;
use light_ctoken_interface::COMPRESSED_TOKEN_PROGRAM_ID;
use solana_pubkey::Pubkey;

use crate::{AnchorDeserialize, AnchorSerialize};

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize, PartialEq)]
pub struct SplInterfacePda {
    pub pubkey: Pubkey,
    pub bump: u8,
    pub index: u8,
}

/// Derive the spl interface pda for a given mint
pub fn get_spl_interface_pda(mint: &Pubkey) -> Pubkey {
    get_spl_interface_pda_with_index(mint, 0)
}

/// Find the spl interface pda for a given mint and index
pub fn find_spl_interface_pda_with_index(mint: &Pubkey, spl_interface_index: u8) -> (Pubkey, u8) {
    let seeds = &[POOL_SEED, mint.as_ref(), &[spl_interface_index]];
    let seeds = if spl_interface_index == 0 {
        &seeds[..2]
    } else {
        &seeds[..]
    };
    Pubkey::find_program_address(seeds, &Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID))
}

/// Get the spl interface pda for a given mint and index
pub fn get_spl_interface_pda_with_index(mint: &Pubkey, spl_interface_index: u8) -> Pubkey {
    find_spl_interface_pda_with_index(mint, spl_interface_index).0
}

/// Derive spl interface pda information for a given mint
pub fn derive_spl_interface_pda(mint: &solana_pubkey::Pubkey, index: u8) -> SplInterfacePda {
    let (pubkey, bump) = find_spl_interface_pda_with_index(mint, index);
    SplInterfacePda {
        pubkey,
        bump,
        index,
    }
}
