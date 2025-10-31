use core::panic::Location;

use crate::{
    checks::{check_mut, check_non_mut, check_signer},
    AccountError, AccountInfoTrait,
};

/// Iterator over accounts that provides detailed error messages when accounts are missing.
///
/// This iterator helps with debugging account setup issues by tracking which accounts
/// are requested and providing clear error messages when there are insufficient accounts.
pub struct AccountIterator<'info, T: AccountInfoTrait> {
    accounts: &'info [T],
    position: usize,
    #[allow(unused)]
    owner: [u8; 32],
}

impl<'info, T: AccountInfoTrait> AccountIterator<'info, T> {
    /// Create a new AccountIterator from a slice of AccountInfo.
    #[inline(always)]
    pub fn new(accounts: &'info [T]) -> Self {
        Self {
            accounts,
            position: 0,
            owner: [0; 32],
        }
    }

    #[inline(always)]
    pub fn new_with_owner(accounts: &'info [T], owner: [u8; 32]) -> Self {
        Self {
            accounts,
            position: 0,
            owner,
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
    #[inline(always)]
    pub fn next_account(&mut self, _account_name: &str) -> Result<&'info T, AccountError> {
        if self.position >= self.accounts.len() {
            #[cfg(all(feature = "msg", feature = "std"))]
            {
                let location = Location::caller();
                solana_msg::msg!(
                    "ERROR: Not enough accounts. Requested '{}' at index {} but only {} accounts available. {}:{}:{}",
                    _account_name, self.position, self.accounts.len(), location.file(), location.line(), location.column()
                );
            }
            return Err(AccountError::NotEnoughAccountKeys);
        }

        let account = &self.accounts[self.position];
        self.position += 1;

        Ok(account)
    }

    #[inline(always)]
    #[track_caller]
    pub fn next_checked_pubkey(
        &mut self,
        account_name: &str,
        pubkey: [u8; 32],
    ) -> Result<&'info T, AccountError> {
        let account_info = self.next_account(account_name)?;
        if account_info.key() != pubkey {
            Err(AccountError::InvalidAccount).inspect_err(|e| {
                self.print_on_error_pubkey(
                    e,
                    account_info.key(),
                    pubkey,
                    account_name,
                    Location::caller(),
                )
            })
        } else {
            Ok(account_info)
        }
    }

    #[inline(always)]
    #[track_caller]
    pub fn next_option(
        &mut self,
        account_name: &str,
        is_some: bool,
    ) -> Result<Option<&'info T>, AccountError> {
        if is_some {
            let account_info = self.next_account(account_name)?;
            Ok(Some(account_info))
        } else {
            Ok(None)
        }
    }

    #[inline(always)]
    #[track_caller]
    pub fn next_option_mut(
        &mut self,
        account_name: &str,
        is_some: bool,
    ) -> Result<Option<&'info T>, AccountError> {
        if is_some {
            let account_info = self.next_mut(account_name)?;
            Ok(Some(account_info))
        } else {
            Ok(None)
        }
    }

    #[inline(always)]
    #[track_caller]
    pub fn next_option_signer(
        &mut self,
        account_name: &str,
        is_some: bool,
    ) -> Result<Option<&'info T>, AccountError> {
        if is_some {
            let account_info = self.next_signer(account_name)?;
            Ok(Some(account_info))
        } else {
            Ok(None)
        }
    }

    #[inline(always)]
    #[track_caller]
    pub fn next_signer_mut(&mut self, account_name: &str) -> Result<&'info T, AccountError> {
        let account_info = self.next_signer(account_name)?;
        check_mut(account_info)
            .inspect_err(|e| self.print_on_error(e, account_name, Location::caller()))?;
        Ok(account_info)
    }

    #[inline(always)]
    #[track_caller]
    pub fn next_signer(&mut self, account_name: &str) -> Result<&'info T, AccountError> {
        let account_info = self.next_account(account_name)?;
        check_signer(account_info)
            .inspect_err(|e| self.print_on_error(e, account_name, Location::caller()))?;
        Ok(account_info)
    }

    #[inline(always)]
    #[track_caller]
    pub fn next_signer_non_mut(&mut self, account_name: &str) -> Result<&'info T, AccountError> {
        let account_info = self.next_signer(account_name)?;
        check_non_mut(account_info)
            .inspect_err(|e| self.print_on_error(e, account_name, Location::caller()))?;
        Ok(account_info)
    }

    #[inline(always)]
    #[track_caller]
    pub fn next_non_mut(&mut self, account_name: &str) -> Result<&'info T, AccountError> {
        let account_info = self.next_account(account_name)?;
        check_non_mut(account_info)
            .inspect_err(|e| self.print_on_error(e, account_name, Location::caller()))?;
        Ok(account_info)
    }

    #[inline(always)]
    #[track_caller]
    pub fn next_mut(&mut self, account_name: &str) -> Result<&'info T, AccountError> {
        let account_info = self.next_account(account_name)?;
        check_mut(account_info)
            .inspect_err(|e| self.print_on_error(e, account_name, Location::caller()))?;
        Ok(account_info)
    }

    /// Get all remaining accounts in the iterator.
    #[inline(always)]
    #[track_caller]
    pub fn remaining(self) -> Result<&'info [T], AccountError> {
        if self.position >= self.accounts.len() {
            #[cfg(all(feature = "msg", feature = "std"))]
            {
                let location = Location::caller();
                let account_name = "remaining accounts";
                solana_msg::msg!(
                    "ERROR: Not enough accounts. Requested '{}' at index {} but only {} accounts available. {}:{}:{}",
                    account_name, self.position, self.accounts.len(), location.file(), location.line(), location.column()
                );
            }
            return Err(AccountError::NotEnoughAccountKeys);
        }
        Ok(&self.accounts[self.position..])
    }

    /// Get all remaining accounts in the iterator without validation.
    ///
    /// Returns an empty slice if position is at or past the end.
    #[inline(always)]
    #[track_caller]
    pub fn remaining_unchecked(self) -> Result<&'info [T], AccountError> {
        if self.position >= self.accounts.len() {
            Ok(&[])
        } else {
            Ok(&self.accounts[self.position..])
        }
    }

    /// Get the current position in the iterator.
    #[inline(always)]
    pub fn position(&self) -> usize {
        self.position
    }

    /// Get the total number of accounts.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    /// Check if the iterator is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }

    #[inline(always)]
    pub fn iterator_is_empty(&self) -> bool {
        self.len() == self.position()
    }

    #[cold]
    fn print_on_error(&self, error: &AccountError, account_name: &str, location: &Location) {
        #[cfg(all(feature = "msg", feature = "std"))]
        solana_msg::msg!(
            "ERROR: {}. for account '{}' at index {}  {}:{}:{}",
            error,
            account_name,
            self.position.saturating_sub(1),
            location.file(),
            location.line(),
            location.column()
        );
        #[cfg(not(all(feature = "msg", feature = "std")))]
        let _ = (error, account_name, location);
    }
    #[cold]
    fn print_on_error_pubkey(
        &self,
        error: &AccountError,
        pubkey1: [u8; 32],
        pubkey2: [u8; 32],
        account_name: &str,
        location: &Location,
    ) {
        #[cfg(all(feature = "msg", feature = "std", feature = "solana"))]
        solana_msg::msg!(
            "ERROR: {}. for account '{}' address: {:?}, expected: {:?}, at index {}  {}:{}:{}",
            error,
            account_name,
            solana_pubkey::Pubkey::new_from_array(pubkey1),
            solana_pubkey::Pubkey::new_from_array(pubkey2),
            self.position.saturating_sub(1),
            location.file(),
            location.line(),
            location.column()
        );
        #[cfg(all(feature = "msg", feature = "std", not(feature = "solana")))]
        solana_msg::msg!(
            "ERROR: {}. for account '{}' address: {:?}, expected: {:?}, at index {}  {}:{}:{}",
            error,
            account_name,
            pubkey1,
            pubkey2,
            self.position.saturating_sub(1),
            location.file(),
            location.line(),
            location.column()
        );
        #[cfg(not(all(feature = "msg", feature = "std")))]
        let _ = (error, pubkey1, pubkey2, account_name, location);
    }
}
