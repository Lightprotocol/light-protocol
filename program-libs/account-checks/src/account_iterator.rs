use std::panic::Location;

use crate::{AccountError, AccountInfoTrait};

/// Iterator over accounts that provides detailed error messages when accounts are missing.
///
/// This iterator helps with debugging account setup issues by tracking which accounts
/// are requested and providing clear error messages when there are insufficient accounts.
pub struct AccountIterator<'info, T: AccountInfoTrait> {
    accounts: &'info [T],
    position: usize,
}

impl<'info, T: AccountInfoTrait> AccountIterator<'info, T> {
    /// Create a new AccountIterator from a slice of AccountInfo.
    pub fn new(accounts: &'info [T]) -> Self {
        Self {
            accounts,
            position: 0,
        }
    }

    /// Get the next account with a descriptive name.
    ///
    /// # Arguments
    /// * `account_name` - A descriptive name for the account being requested (for debugging)
    ///
    /// # Returns
    /// * `Ok(&T)` - The next account in the iterator
    /// * `Err(AccountError::NotEnoughAccountKeys)` - If no more accounts are available
    #[track_caller]
    pub fn next_account(&mut self, account_name: &str) -> Result<&'info T, AccountError> {
        let location = Location::caller();

        if self.position >= self.accounts.len() {
            solana_msg::msg!(
                "ERROR: Not enough accounts. Requested '{}' at index {} but only {} accounts available. {}:{}:{}",
                account_name, self.position, self.accounts.len(), location.file(), location.line(), location.column()
            );
            return Err(AccountError::NotEnoughAccountKeys);
        }

        let account = &self.accounts[self.position];
        self.position += 1;

        Ok(account)
    }

    /// Get all remaining accounts in the iterator.
    #[track_caller]
    pub fn remaining(&self) -> Result<&'info [T], AccountError> {
        let location = Location::caller();
        if self.position >= self.accounts.len() {
            let account_name = "remaining accounts";
            solana_msg::msg!(
                "ERROR: Not enough accounts. Requested '{}' at index {} but only {} accounts available. {}:{}:{}",
                account_name, self.position, self.accounts.len(), location.file(), location.line(), location.column()
            );
            return Err(AccountError::NotEnoughAccountKeys);
        }
        Ok(&self.accounts[self.position..])
    }

    /// Get the current position in the iterator.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Get the total number of accounts.
    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    /// Check if the iterator is empty.
    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }
}
