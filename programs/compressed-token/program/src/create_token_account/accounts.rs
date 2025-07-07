use anchor_lang::prelude::{AccountInfo, ProgramError};
use light_account_checks::checks::{check_mut, check_non_mut, check_signer};

pub struct CreateTokenAccountAccounts<'a, 'info> {
    pub token_account: &'a AccountInfo<'info>,
    pub mint: &'a AccountInfo<'info>,
    pub fee_payer: &'a AccountInfo<'info>,
    pub system_program: &'a AccountInfo<'info>,
}

impl<'a, 'info> CreateTokenAccountAccounts<'a, 'info> {
    pub fn new(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        Ok(Self {
            token_account: &accounts[0],
            mint: &accounts[1],
            fee_payer: &accounts[2],
            system_program: &accounts[3],
        })
    }

    pub fn get_checked(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let accounts_struct = Self::new(accounts)?;

        // Basic validations using light_account_checks
        check_signer(accounts_struct.fee_payer)?;
        check_mut(accounts_struct.fee_payer)?;
        check_mut(accounts_struct.token_account)?;
        check_non_mut(accounts_struct.mint)?;
        check_non_mut(accounts_struct.system_program)?;

        Ok(accounts_struct)
    }
}