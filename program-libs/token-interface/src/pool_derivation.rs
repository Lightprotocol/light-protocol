//! SPL interface PDA derivation utilities for Light Protocol.
//!
//! This module provides functions to derive SPL interface PDAs (token pools) for both regular
//! and restricted mints. Restricted mints (those with Pausable, PermanentDelegate,
//! TransferFeeConfig, TransferHook, or DefaultAccountState extensions) use a different derivation path
//! to prevent accidental compression via legacy anchor instructions.

use solana_pubkey::Pubkey;
use spl_token_2022::{
    extension::{BaseStateWithExtensions, PodStateWithExtensions},
    pod::PodMint,
};

use crate::{
    constants::{LIGHT_TOKEN_PROGRAM_ID, POOL_SEED, RESTRICTED_POOL_SEED},
    is_restricted_extension,
};

/// Maximum number of pool accounts per mint.
pub const NUM_MAX_POOL_ACCOUNTS: u8 = 5;

// ============================================================================
// SPL interface PDA derivation (uses LIGHT_TOKEN_PROGRAM_ID)
// ============================================================================

/// Find the SPL interface PDA for a given mint (index 0).
///
/// # Arguments
/// * `mint` - The mint public key
/// * `restricted` - Whether to use restricted derivation (for mints with restricted extensions)
///
/// # Seed format
/// - Regular: `["pool", mint]`
/// - Restricted: `["pool", mint, "restricted"]`
pub fn find_spl_interface_pda(mint: &Pubkey, restricted: bool) -> (Pubkey, u8) {
    find_spl_interface_pda_with_index(mint, 0, restricted)
}

/// Find the SPL interface PDA for a given mint and index.
///
/// # Arguments
/// * `mint` - The mint public key
/// * `index` - The pool index (0-4)
/// * `restricted` - Whether to use restricted derivation (for mints with restricted extensions)
///
/// # Seed format
/// - Regular index 0: `["pool", mint]`
/// - Regular index 1-4: `["pool", mint, index]`
/// - Restricted index 0: `["pool", mint, "restricted"]`
/// - Restricted index 1-4: `["pool", mint, "restricted", index]`
pub fn find_spl_interface_pda_with_index(
    mint: &Pubkey,
    index: u8,
    restricted: bool,
) -> (Pubkey, u8) {
    let program_id = Pubkey::from(LIGHT_TOKEN_PROGRAM_ID);
    let index_bytes = [index];

    let seeds: &[&[u8]] = if restricted {
        if index == 0 {
            &[POOL_SEED, mint.as_ref(), RESTRICTED_POOL_SEED]
        } else {
            &[POOL_SEED, mint.as_ref(), RESTRICTED_POOL_SEED, &index_bytes]
        }
    } else if index == 0 {
        &[POOL_SEED, mint.as_ref()]
    } else {
        &[POOL_SEED, mint.as_ref(), &index_bytes]
    };

    Pubkey::find_program_address(seeds, &program_id)
}

/// Get the SPL interface PDA address for a given mint (index 0).
pub fn get_spl_interface_pda(mint: &Pubkey, restricted: bool) -> Pubkey {
    find_spl_interface_pda(mint, restricted).0
}

// ============================================================================
// Validation
// ============================================================================

/// Validate that an SPL interface PDA is correctly derived.
///
/// # Arguments
/// * `mint_bytes` - The mint public key as bytes
/// * `spl_interface_pubkey` - The SPL interface PDA to validate
/// * `pool_index` - The pool index (0-4)
/// * `bump` - Optional bump seed for faster validation
/// * `restricted` - Whether to validate against restricted derivation
///
/// # Returns
/// `true` if the PDA is valid, `false` otherwise
#[inline(always)]
pub fn is_valid_spl_interface_pda(
    mint_bytes: &[u8],
    spl_interface_pubkey: &Pubkey,
    pool_index: u8,
    bump: Option<u8>,
    restricted: bool,
) -> bool {
    let program_id = Pubkey::from(LIGHT_TOKEN_PROGRAM_ID);
    let index_bytes = [pool_index];

    let pda = if let Some(bump) = bump {
        // Fast path: use provided bump to derive address directly
        let bump_bytes = [bump];
        let seeds: &[&[u8]] = if restricted {
            if pool_index == 0 {
                &[POOL_SEED, mint_bytes, RESTRICTED_POOL_SEED, &bump_bytes]
            } else {
                &[
                    POOL_SEED,
                    mint_bytes,
                    RESTRICTED_POOL_SEED,
                    &index_bytes,
                    &bump_bytes,
                ]
            }
        } else if pool_index == 0 {
            &[POOL_SEED, mint_bytes, &bump_bytes]
        } else {
            &[POOL_SEED, mint_bytes, &index_bytes, &bump_bytes]
        };

        match Pubkey::create_program_address(seeds, &program_id) {
            Ok(pda) => pda,
            Err(_) => return false,
        }
    } else {
        // Slow path: find program address
        let seeds: &[&[u8]] = if restricted {
            if pool_index == 0 {
                &[POOL_SEED, mint_bytes, RESTRICTED_POOL_SEED]
            } else {
                &[POOL_SEED, mint_bytes, RESTRICTED_POOL_SEED, &index_bytes]
            }
        } else if pool_index == 0 {
            &[POOL_SEED, mint_bytes]
        } else {
            &[POOL_SEED, mint_bytes, &index_bytes]
        };

        Pubkey::find_program_address(seeds, &program_id).0
    };

    pda == *spl_interface_pubkey
}

// ============================================================================
// Mint extension helpers
// ============================================================================

/// Check if a mint has any restricted extensions.
///
/// Restricted extensions (Pausable, PermanentDelegate, TransferFeeConfig, TransferHook, DefaultAccountState)
/// require using the restricted pool derivation path.
///
/// # Arguments
/// * `mint_data` - The raw mint account data
///
/// # Returns
/// `true` if the mint has any restricted extensions, `false` otherwise
pub fn has_restricted_extensions(mint_data: &[u8]) -> bool {
    let mint = match PodStateWithExtensions::<PodMint>::unpack(mint_data) {
        Ok(mint) => mint,
        Err(_) => return false,
    };

    let extensions = match mint.get_extension_types() {
        Ok(exts) => exts,
        Err(_) => return false,
    };

    extensions.iter().any(is_restricted_extension)
}
