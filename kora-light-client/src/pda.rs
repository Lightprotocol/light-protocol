//! PDA derivation helpers for Light Protocol.
//!
//! Ported from `sdk-libs/token-sdk/src/utils.rs`.

use solana_pubkey::Pubkey;

use crate::program_ids::{
    LIGHT_TOKEN_PROGRAM_ID, POOL_SEED, SPL_TOKEN_2022_PROGRAM_ID, SPL_TOKEN_PROGRAM_ID,
};

/// Returns the Light Token associated token address for a given owner and mint.
pub fn get_associated_token_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    get_associated_token_address_and_bump(owner, mint).0
}

/// Returns the Light Token associated token address and bump for a given owner and mint.
pub fn get_associated_token_address_and_bump(owner: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            &owner.to_bytes(),
            &LIGHT_TOKEN_PROGRAM_ID.to_bytes(),
            &mint.to_bytes(),
        ],
        &LIGHT_TOKEN_PROGRAM_ID,
    )
}

/// Returns the SPL interface PDA, bump, and pool index for a given mint.
///
/// Tries pool_index 0 first (most common). If the PDA derivation is needed
/// for other pool indices, use `find_spl_interface_pda_with_index`.
pub fn find_spl_interface_pda(mint: &Pubkey) -> (Pubkey, u8) {
    find_spl_interface_pda_with_index(mint, 0)
}

/// Returns the SPL interface PDA and bump for a given mint and pool index.
pub fn find_spl_interface_pda_with_index(mint: &Pubkey, pool_index: u8) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[POOL_SEED, &mint.to_bytes(), &[pool_index]],
        &LIGHT_TOKEN_PROGRAM_ID,
    )
}

/// Derive the CPI authority PDA for the Light Token Program.
pub fn derive_cpi_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[crate::program_ids::CPI_AUTHORITY_PDA_SEED],
        &LIGHT_TOKEN_PROGRAM_ID,
    )
}

/// Check if an account owner is a Light token program.
///
/// Returns `true` if owner is `LIGHT_TOKEN_PROGRAM_ID`.
/// Returns `false` if owner is SPL Token or Token-2022.
/// Returns `None` if owner is unrecognized.
pub fn is_light_token_owner(owner: &Pubkey) -> Option<bool> {
    if owner == &LIGHT_TOKEN_PROGRAM_ID {
        Some(true)
    } else if owner == &SPL_TOKEN_PROGRAM_ID || owner == &SPL_TOKEN_2022_PROGRAM_ID {
        Some(false)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpi_authority_pda_matches_known() {
        let (pda, bump) = derive_cpi_authority_pda();
        assert_eq!(pda, crate::program_ids::CPI_AUTHORITY_PDA);
        assert_eq!(bump, crate::program_ids::BUMP_CPI_AUTHORITY);
    }
}
