pub mod access;
pub mod errors;
pub mod events;
pub mod fee;
pub mod merkle_tree;
pub mod queue;
pub mod rollover;
pub mod utils;

#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
pub(crate) use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use light_compressed_account::{QueueType, TreeType};
// Pinocchio imports
#[allow(unused_imports)]
#[cfg(feature = "pinocchio")]
pub(crate) use pinocchio::{
    msg, program_error::ProgramError, sysvars::clock::Clock, sysvars::Sysvar,
};
// Solana imports (default)
#[allow(unused_imports)]
#[cfg(not(feature = "pinocchio"))]
pub(crate) use solana_program::{clock::Clock, msg, program_error::ProgramError, sysvar::Sysvar};
