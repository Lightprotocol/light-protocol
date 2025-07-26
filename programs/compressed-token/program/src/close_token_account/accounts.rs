use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::checks::{check_mut, check_signer};
use pinocchio::account_info::AccountInfo;

use crate::shared::AccountIterator;

pub struct CloseTokenAccountAccounts<'info> {
    pub token_account: &'info AccountInfo,
    pub destination: &'info AccountInfo,
    pub authority: &'info AccountInfo,
}

impl<'info> CloseTokenAccountAccounts<'info> {
    pub fn validate_and_parse(accounts: &'info [AccountInfo]) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);

        let token_account = iter.next_account()?;
        let destination = iter.next_account()?;
        let authority = iter.next_account()?;

        // Basic validations using light_account_checks
        check_mut(token_account)?;
        check_mut(destination)?;
        check_signer(authority)?;

        Ok(CloseTokenAccountAccounts {
            token_account,
            destination,
            authority,
        })
    }
}
