mod config;
pub mod v1;
#[cfg(feature = "v2")]
pub mod v2;

pub use config::CpiAccountsConfig;
use light_account_checks::AccountInfoTrait;

use crate::error::Result;

/// Trait for CPI accounts that provide access to tree accounts
pub trait TreeAccounts<T: AccountInfoTrait + Clone> {
    fn get_tree_account_info(&self, tree_index: usize) -> Result<&T>;
}
