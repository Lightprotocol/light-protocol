#![cfg_attr(not(feature = "std"), no_std)]

pub mod account_info;
pub mod account_iterator;
pub mod checks;
pub mod discriminator;
pub mod error;
pub mod packed_accounts;

pub use account_info::account_info_trait::AccountInfoTrait;
pub use account_iterator::AccountIterator;
pub use error::AccountError;
