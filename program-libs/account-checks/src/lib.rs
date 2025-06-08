#[cfg(any(feature = "pinocchio", feature = "solana"))]
pub mod checks;
pub mod discriminator;
pub mod error;
pub mod test_account_info;

#[cfg(feature = "pinocchio")]
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};
#[cfg(feature = "solana")]
use {solana_account_info::AccountInfo, solana_pubkey::Pubkey};
