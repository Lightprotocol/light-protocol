//! Create CToken ATA CPI builder for pinocchio.
//!
//! Re-exports the generic `CreateTokenAtaCpi` from `light_sdk_types`
//! specialized for pinocchio's `AccountInfo`.

use light_account_checks::AccountInfoTrait;
// TODO: add types with generics set so that we dont expose the generics
pub use light_sdk_types::interface::cpi::create_token_accounts::{
    CreateTokenAtaCpi, CreateTokenAtaCpiIdempotent, CreateTokenAtaRentFreeCpi,
};
use light_token_interface::LIGHT_TOKEN_PROGRAM_ID;
use pinocchio::account_info::AccountInfo;

/// Derive the associated token account address for a given owner and mint.
///
/// Returns `([u8; 32], u8)` -- the ATA address and bump seed.
///
/// Uses pinocchio's `AccountInfo` for PDA derivation.
pub fn derive_associated_token_account(owner: &[u8; 32], mint: &[u8; 32]) -> ([u8; 32], u8) {
    AccountInfo::find_program_address(
        &[
            owner.as_ref(),
            LIGHT_TOKEN_PROGRAM_ID.as_ref(),
            mint.as_ref(),
        ],
        &LIGHT_TOKEN_PROGRAM_ID,
    )
}
