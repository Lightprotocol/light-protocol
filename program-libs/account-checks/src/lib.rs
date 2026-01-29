//! # light-account-checks
//!
//! Checks for Solana accounts.
//!
//! | Module | Description |
//! |--------|-------------|
//! | [`AccountInfoTrait`] | Trait abstraction over Solana account info types |
//! | [`AccountIterator`] | Iterates over a slice of accounts by index |
//! | [`AccountError`] | Error type for account validation failures |
//! | [`checks`] | Owner, signer, writable, and rent-exempt checks |
//! | [`discriminator`] | Account discriminator constants and validation |
//! | [`packed_accounts`] | Packed account struct deserialization |

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
