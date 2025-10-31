#[cfg(all(feature = "msg", feature = "std"))]
use core::panic::Location;

use crate::{AccountError, AccountInfoTrait};

/// Dynamic accounts slice for index-based access
/// Contains mint, owner, delegate, merkle tree, and queue accounts
pub struct ProgramPackedAccounts<'info, A: AccountInfoTrait> {
    pub accounts: &'info [A],
}

impl<A: AccountInfoTrait> ProgramPackedAccounts<'_, A> {
    /// Get account by index with bounds checking
    #[track_caller]
    #[inline(never)]
    pub fn get(&self, index: usize, _name: &str) -> Result<&A, AccountError> {
        if index >= self.accounts.len() {
            #[cfg(all(feature = "msg", feature = "std"))]
            {
                let location = Location::caller();
                solana_msg::msg!(
                    "ERROR: Not enough accounts. Requested '{}' at index {} but only {} accounts available. {}:{}:{}",
                    _name, index, self.accounts.len(), location.file(), location.line(), location.column()
                );
            }
            return Err(AccountError::NotEnoughAccountKeys);
        }
        Ok(&self.accounts[index])
    }

    // TODO: add get_checked_account from  PackedAccounts.
    /// Get account by u8 index with bounds checking
    #[track_caller]
    #[inline(never)]
    pub fn get_u8(&self, index: u8, name: &str) -> Result<&A, AccountError> {
        self.get(index as usize, name)
    }
}
