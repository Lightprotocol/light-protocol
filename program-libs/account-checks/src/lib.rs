pub mod checks;
pub mod discriminator;
pub mod error;

#[cfg(feature = "pinocchio")]
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
#[cfg(all(feature = "pinocchio", target_os = "solana"))]
use pinocchio::{sysvars::rent::Rent, sysvars::Sysvar};
#[cfg(not(feature = "pinocchio"))]
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
#[cfg(all(not(feature = "pinocchio"), target_os = "solana"))]
use solana_program::{msg, rent::Rent, sysvar::Sysvar};
