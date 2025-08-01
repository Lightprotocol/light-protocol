use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::checks::{check_mut, check_non_mut};
use pinocchio::account_info::AccountInfo;

use crate::shared::AccountIterator;

pub struct CreateTokenAccountAccounts<'info> {
    pub token_account: &'info AccountInfo,
    pub mint: &'info AccountInfo,
}

impl<'info> CreateTokenAccountAccounts<'info> {
    pub fn validate_and_parse(accounts: &'info [AccountInfo]) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);

        let token_account = iter.next_account("token_account")?;
        let mint = iter.next_account("mint")?;

        check_mut(token_account)?;
        check_non_mut(mint)?;

        Ok(CreateTokenAccountAccounts {
            token_account,
            mint,
        })
    }
}
