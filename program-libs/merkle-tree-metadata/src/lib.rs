pub mod access;
pub mod errors;
pub mod events;
pub mod fee;
pub mod merkle_tree;
pub mod queue;
pub mod rollover;
pub mod utils;

pub use light_compressed_account::{QueueType, TreeType};

#[allow(unused_imports)]
#[cfg(feature = "solana")]
use solana_program::{clock::Clock, msg, program_error::ProgramError, sysvar::Sysvar};

#[cfg(all(
    feature = "anchor",
    not(feature = "solana"),
    not(feature = "pinocchio")
))]
use anchor_lang::solana_program::{clock::Clock, msg, program_error::ProgramError, sysvar::Sysvar};
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};

#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};

#[cfg(all(
    feature = "pinocchio",
    not(feature = "solana"),
    not(feature = "solana")
))]
use pinocchio::{clock::Clock, msg, program_error::ProgramError, sysvar::Sysvar};
