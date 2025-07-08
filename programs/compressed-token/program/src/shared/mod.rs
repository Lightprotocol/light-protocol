pub mod context;
pub mod cpi;
pub mod cpi_bytes_size;
pub mod inputs;
pub mod outputs;
pub mod initialize_token_account;

use anchor_lang::solana_program::program_error::ProgramError;
use pinocchio::account_info::AccountInfo;

pub struct AccountIterator<'info> {
    accounts: &'info [AccountInfo],
    position: usize,
}

impl<'info> AccountIterator<'info> {
    pub fn new(accounts: &'info [AccountInfo]) -> Self {
        Self {
            accounts,
            position: 0,
        }
    }

    pub fn next(&mut self) -> Result<&'info AccountInfo, ProgramError> {
        if self.position >= self.accounts.len() {
            return Err(ProgramError::NotEnoughAccountKeys);
        }
        let account = &self.accounts[self.position];
        self.position += 1;
        Ok(account)
    }

    pub fn remaining(&self) -> &'info [AccountInfo] {
        &self.accounts[self.position..]
    }
}
