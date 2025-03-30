pub mod checks;
pub mod discriminator;
pub mod error;

#[cfg(all(feature = "anchor_lang", target_os = "solana"))]
use anchor_lang::solana_program::{msg, rent::Rent, sysvar::Sysvar};
#[cfg(all(
    feature = "anchor",
    not(feature = "solana"),
    not(feature = "pinocchio")
))]
use anchor_lang::{
    prelude::Pubkey,
    solana_program::{
        account_info::AccountInfo, program_error::ProgramError, rent::Rent, sysvar::Sysvar,
    },
};

#[cfg(feature = "solana")]
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

#[cfg(all(feature = "solana", target_os = "solana"))]
use solana_program::{msg, rent::Rent, sysvar::Sysvar};

#[cfg(all(
    feature = "pinocchio",
    not(feature = "solana"),
    not(feature = "anchor")
))]
use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, rent::Rent,
    sysvar::Sysvar,
};
#[cfg(all(feature = "pinocchio", target_os = "solana"))]
use pinocchio::{msg, rent::Rent, sysvar::Sysvar};
