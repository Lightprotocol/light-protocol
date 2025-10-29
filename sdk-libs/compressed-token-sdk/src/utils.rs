use solana_account_info::AccountInfo;
use spl_pod::bytemuck::pod_from_bytes;
use spl_token_2022::pod::PodAccount;

use crate::error::TokenSdkError;

/// Get token account balance from account info
pub fn get_token_account_balance(token_account_info: &AccountInfo) -> Result<u64, TokenSdkError> {
    let token_account_data = token_account_info
        .try_borrow_data()
        .map_err(|_| TokenSdkError::AccountBorrowFailed)?;

    // Use zero-copy PodAccount to access the token account
    let pod_account = pod_from_bytes::<PodAccount>(&token_account_data)
        .map_err(|_| TokenSdkError::InvalidAccountData)?;

    Ok(pod_account.amount.into())
}

/// Evaluate if an account is a CToken account
///
/// Returns true if owned by CToken program, false if owned by SPL Token or
/// Token-2022.
pub fn is_ctoken_account(account_info: &AccountInfo) -> Result<bool, TokenSdkError> {
    let ctoken_program_id = Pubkey::from(CTOKEN_PROGRAM_ID);

    if account_info.owner == &ctoken_program_id {
        return Ok(true);
    }

    let token_22 = spl_token_2022::ID;
    let spl_token = Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

    if account_info.owner == &token_22 || account_info.owner == &spl_token {
        return Ok(false);
    }

    // Must be one of the three.
    Err(TokenSdkError::CannotDetermineAccountType)
}
