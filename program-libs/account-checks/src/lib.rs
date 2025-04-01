pub mod checks;
pub mod discriminator;
pub mod error;

// Compile-time check ensuring exactly one feature is active.
// Compile-time exclusivity checks
const _: () = {
    #[cfg(any(
        all(feature = "solana", feature = "anchor"),
        all(feature = "solana", feature = "pinocchio"),
        all(feature = "anchor", feature = "pinocchio")
    ))]
    compile_error!("Only one feature among 'solana', 'anchor', and 'pinocchio' may be active.");
    #[cfg(not(any(feature = "solana", feature = "anchor", feature = "pinocchio")))]
    compile_error!("Exactly one of 'solana', 'anchor', or 'pinocchio' must be enabled.");
};

#[cfg(all(feature = "anchor_lang", target_os = "solana"))]
use anchor_lang::solana_program::{msg, rent::Rent, sysvar::Sysvar};
#[cfg(all(
    feature = "anchor",
    not(feature = "solana"),
    not(feature = "pinocchio")
))]
use anchor_lang::{
    prelude::{ProgramError, Pubkey},
    solana_program::{account_info::AccountInfo, rent::Rent, sysvar::Sysvar},
};

#[cfg(all(
    feature = "solana",
    not(feature = "anchor"),
    not(feature = "pinocchio")
))]
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

#[cfg(all(feature = "solana", target_os = "solana"))]
use solana_program::{msg, rent::Rent, sysvar::Sysvar};

#[cfg(all(
    feature = "pinocchio",
    not(feature = "solana"),
    not(feature = "anchor")
))]
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
#[cfg(all(feature = "pinocchio", target_os = "solana"))]
use pinocchio::{msg, sysvars::rent::Rent, sysvars::Sysvar};
