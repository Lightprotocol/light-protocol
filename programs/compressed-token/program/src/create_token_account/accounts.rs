use anchor_lang::prelude::ProgramError;
use light_account_checks::checks::{check_mut, check_non_mut};
use pinocchio::account_info::AccountInfo;

pub struct CreateTokenAccountAccounts<'a> {
    pub token_account: &'a AccountInfo,
    pub mint: &'a AccountInfo,
}

impl<'a> CreateTokenAccountAccounts<'a> {
    pub fn new(accounts: &'a [AccountInfo]) -> Result<Self, ProgramError> {
        Ok(Self {
            token_account: &accounts[0],
            mint: &accounts[1],
        })
    }

    pub fn get_checked(accounts: &'a [AccountInfo]) -> Result<Self, ProgramError> {
        let accounts_struct = Self::new(accounts)?;

        // Basic validations using light_account_checks
        check_mut(accounts_struct.token_account)?;
        check_non_mut(accounts_struct.mint)?;

        Ok(accounts_struct)
    }
}