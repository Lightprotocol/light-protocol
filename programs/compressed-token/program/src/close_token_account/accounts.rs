use anchor_lang::prelude::ProgramError;
use light_account_checks::checks::{check_mut, check_signer};
use pinocchio::account_info::AccountInfo;

pub struct CloseTokenAccountAccounts<'a> {
    pub token_account: &'a AccountInfo,
    pub destination: &'a AccountInfo,
    pub authority: &'a AccountInfo,
}

impl<'a> CloseTokenAccountAccounts<'a> {
    pub fn new(accounts: &'a [AccountInfo]) -> Result<Self, ProgramError> {
        Ok(Self {
            token_account: &accounts[0],
            destination: &accounts[1],
            authority: &accounts[2],
        })
    }

    pub fn get_checked(accounts: &'a [AccountInfo]) -> Result<Self, ProgramError> {
        let accounts_struct = Self::new(accounts)?;

        // Basic validations using light_account_checks
        check_mut(accounts_struct.token_account)?;
        check_mut(accounts_struct.destination)?;
        check_signer(accounts_struct.authority)?;

        Ok(accounts_struct)
    }
}