use light_sdk_types::CTOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_pubkey::Pubkey;
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

/// Check if an account is a compressed token account
///
/// Returns true if the account is owned by the compressed token program,
/// false if it's owned by SPL Token or Token-2022 program.
pub fn is_ctoken_account(account_info: &AccountInfo) -> Result<bool, TokenSdkError> {
    let ctoken_program_id = Pubkey::from(CTOKEN_PROGRAM_ID);

    // Check if owned by compressed token program
    if account_info.owner == &ctoken_program_id {
        return Ok(true);
    }

    // Check if owned by SPL Token or Token-2022
    let spl_token_program = spl_token_2022::ID;
    let spl_token_legacy = Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

    if account_info.owner == &spl_token_program || account_info.owner == &spl_token_legacy {
        return Ok(false);
    }

    // Unknown account type
    Err(TokenSdkError::CannotDetermineAccountType)
}

/// Check if an account is an SPL token account
///
/// Returns true if the account is owned by SPL Token or Token-2022 program.
pub fn is_spl_token_account(account_info: &AccountInfo) -> Result<bool, TokenSdkError> {
    is_ctoken_account(account_info).map(|is_ctoken| !is_ctoken)
}
