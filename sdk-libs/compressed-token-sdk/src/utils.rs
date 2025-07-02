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
