//! SPL interface PDA derivation utilities.
//!
//! Re-exports from `light_compressed_token_sdk::spl_interface` with additional helpers.

// Re-export everything from compressed-token-sdk spl_interface
pub use light_compressed_token_sdk::spl_interface::*;
use light_token_types::POOL_SEED;
use solana_pubkey::Pubkey;

use crate::instruction::LIGHT_TOKEN_PROGRAM_ID;

/// Get the SPL interface PDA and bump for a given mint (index 0, non-restricted).
pub fn get_spl_interface_pda_and_bump(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[POOL_SEED, mint.as_ref()], &LIGHT_TOKEN_PROGRAM_ID)
}
