use anchor_lang::prelude::{AccountInfo, ProgramError};
use light_account_checks::checks::{check_mut, check_signer};

pub struct CloseTokenAccountAccounts<'a, 'info> {
    pub token_account: &'a AccountInfo<'info>,
    pub destination: &'a AccountInfo<'info>,
    pub authority: &'a AccountInfo<'info>,
}

impl<'a, 'info> CloseTokenAccountAccounts<'a, 'info> {
    pub fn new(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        Ok(Self {
            token_account: &accounts[0],
            destination: &accounts[1],
            authority: &accounts[2],
        })
    }

    pub fn get_checked(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let accounts_struct = Self::new(accounts)?;

        // Basic validations using light_account_checks
        check_mut(accounts_struct.token_account)?;
        check_mut(accounts_struct.destination)?;
        check_signer(accounts_struct.authority)?;

        Ok(accounts_struct)
    }
}