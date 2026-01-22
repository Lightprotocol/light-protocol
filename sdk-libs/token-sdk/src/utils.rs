//! Utility functions and default account configurations.

// Re-export TokenDefaultAccounts from compressed-token-sdk
pub use light_compressed_token_sdk::utils::TokenDefaultAccounts;
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token_interface::state::Token;
use solana_account_info::AccountInfo;
use solana_pubkey::Pubkey;

use crate::{constants::LIGHT_TOKEN_PROGRAM_ID as PROGRAM_ID, error::TokenSdkError};

/// Returns the associated token address for a given owner and mint.
pub fn get_associated_token_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    get_associated_token_address_and_bump(owner, mint).0
}

/// Returns the associated token address and bump for a given owner and mint.
pub fn get_associated_token_address_and_bump(owner: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[&owner.to_bytes(), &PROGRAM_ID.to_bytes(), &mint.to_bytes()],
        &PROGRAM_ID,
    )
}

/// Get the token balance from a Light token account.
pub fn get_token_account_balance(token_account_info: &AccountInfo) -> Result<u64, TokenSdkError> {
    let data = token_account_info
        .try_borrow_data()
        .map_err(|_| TokenSdkError::AccountBorrowFailed)?;
    Token::amount_from_slice(&data).map_err(|_| TokenSdkError::InvalidAccountData)
}

/// Check if an account owner is a Light token program.
///
/// Returns `Ok(true)` if owner is `LIGHT_TOKEN_PROGRAM_ID`.
/// Returns `Ok(false)` if owner is SPL Token or Token-2022.
/// Returns `Err` if owner is unrecognized.
pub fn is_light_token_owner(owner: &Pubkey) -> Result<bool, TokenSdkError> {
    let light_token_program_id = Pubkey::from(LIGHT_TOKEN_PROGRAM_ID);

    if owner == &light_token_program_id {
        return Ok(true);
    }

    let spl_token = Pubkey::from(light_token_types::SPL_TOKEN_PROGRAM_ID);
    let spl_token_2022 = Pubkey::from(light_token_types::SPL_TOKEN_2022_PROGRAM_ID);

    if owner == &spl_token_2022 || owner == &spl_token {
        return Ok(false);
    }

    Err(TokenSdkError::CannotDetermineAccountType)
}

/// Check if an account is a Light token account (by checking its owner).
///
/// Returns `Ok(true)` if owner is `LIGHT_TOKEN_PROGRAM_ID`.
/// Returns `Ok(false)` if owner is SPL Token or Token-2022.
/// Returns `Err` if owner is unrecognized.
pub fn is_token_account(account_info: &AccountInfo) -> Result<bool, TokenSdkError> {
    is_light_token_owner(account_info.owner)
}
