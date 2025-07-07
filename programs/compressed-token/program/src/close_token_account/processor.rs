use anchor_lang::prelude::{AccountInfo, ProgramError};
use anchor_lang::solana_program::pubkey::Pubkey;
use spl_pod::bytemuck::pod_from_bytes;
use spl_token_2022::pod::PodAccount;
use spl_token_2022::state::AccountState;

use super::accounts::CloseTokenAccountAccounts;

/// Process the close token account instruction
pub fn process_close_token_account<'info>(
    account_infos: &'info [AccountInfo<'info>],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Validate and get accounts
    let accounts = CloseTokenAccountAccounts::get_checked(account_infos)?;

    // Validate token account state and balance
    {
        let token_account_data = accounts.token_account.try_borrow_data()?;
        let pod_account = pod_from_bytes::<PodAccount>(&token_account_data)
            .map_err(|_| ProgramError::InvalidAccountData)?;

        // Check that the account is initialized
        if pod_account.state != AccountState::Initialized as u8 {
            return Err(ProgramError::UninitializedAccount);
        }

        // Check that the account has zero balance
        let balance: u64 = pod_account.amount.into();
        if balance != 0 {
            return Err(ProgramError::InvalidAccountData);
        }

        // Verify the authority matches the account owner
        let account_owner = Pubkey::from(pod_account.owner);
        if account_owner != *accounts.authority.key {
            return Err(ProgramError::InvalidAccountOwner);
        }
    }

    // Transfer all lamports from token account to destination
    let token_account_lamports = accounts.token_account.lamports();
    **accounts.token_account.try_borrow_mut_lamports()? = 0;
    **accounts.destination.try_borrow_mut_lamports()? = accounts
        .destination
        .lamports()
        .checked_add(token_account_lamports)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Clear the token account data
    let mut token_account_data = accounts.token_account.try_borrow_mut_data()?;
    token_account_data.fill(0);

    Ok(())
}