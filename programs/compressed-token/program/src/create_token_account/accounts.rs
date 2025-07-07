use anchor_lang::prelude::{AccountInfo, ProgramError};
use light_account_checks::checks::{check_mut, check_non_mut};

pub struct CreateTokenAccountAccounts<'a, 'info> {
    pub token_account: &'a AccountInfo<'info>,
    pub mint: &'a AccountInfo<'info>,
}

impl<'a, 'info> CreateTokenAccountAccounts<'a, 'info> {
    pub fn new(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        Ok(Self {
            token_account: &accounts[0],
            mint: &accounts[1],
        })
    }

    pub fn get_checked(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let accounts_struct = Self::new(accounts)?;

        // Basic validations using light_account_checks
        check_mut(accounts_struct.token_account)?;
        check_non_mut(accounts_struct.mint)?;

        Ok(accounts_struct)
    }
}