use anchor_lang::prelude::ProgramError;
use light_account_checks::AccountInfoTrait;
use pinocchio::account_info::AccountInfo;
use spl_pod::bytemuck::pod_from_bytes;
use spl_token_2022::pod::PodAccount;
use spl_token_2022::state::AccountState;

use super::accounts::CloseTokenAccountAccounts;

/// Process the close token account instruction
pub fn process_close_token_account<'info>(
    account_infos: &'info [AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Validate and get accounts
    let accounts = CloseTokenAccountAccounts::get_checked(account_infos)?;

    // Validate token account state and balance
    {
        let token_account_data = AccountInfoTrait::try_borrow_data(accounts.token_account)
            .map_err(|_| ProgramError::InvalidAccountData)?;
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
        let account_owner = solana_pubkey::Pubkey::from(pod_account.owner);
        let authority_key = solana_pubkey::Pubkey::new_from_array(*accounts.authority.key());
        if account_owner != authority_key {
            return Err(ProgramError::InvalidAccountOwner);
        }
    }

    // Transfer all lamports from token account to destination
    let token_account_lamports = AccountInfoTrait::lamports(accounts.token_account);

    // Set token account lamports to 0
    unsafe {
        *accounts.token_account.borrow_mut_lamports_unchecked() = 0;
    }

    // Add lamports to destination
    let destination_lamports = AccountInfoTrait::lamports(accounts.destination);
    let new_destination_lamports = destination_lamports
        .checked_add(token_account_lamports)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    unsafe {
        *accounts.destination.borrow_mut_lamports_unchecked() = new_destination_lamports;
    }

    // Clear the token account data
    let mut token_account_data = AccountInfoTrait::try_borrow_mut_data(accounts.token_account)
        .map_err(|_| ProgramError::InvalidAccountData)?;
    token_account_data.fill(0);

    Ok(())
}
