pub mod access;
pub mod errors;
pub mod events;
pub mod fee;
pub mod merkle_tree;
pub mod queue;
pub mod rollover;
pub mod utils;

#[allow(unused_imports)]
#[cfg(all(
    feature = "anchor",
    not(feature = "solana"),
    not(feature = "pinocchio")
))]
pub(crate) use anchor_lang::solana_program::{
    clock::Clock, msg, program_error::ProgramError, sysvar::Sysvar,
};
#[cfg(feature = "anchor")]
pub(crate) use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
pub(crate) use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use light_compressed_account::{QueueType, TreeType};
#[allow(unused_imports)]
#[cfg(all(
    feature = "pinocchio",
    not(feature = "solana"),
    not(feature = "anchor")
))]
pub(crate) use pinocchio::{
    msg, program_error::ProgramError, sysvars::clock::Clock, sysvars::Sysvar,
};
#[allow(unused_imports)]
#[cfg(feature = "solana")]
pub(crate) use solana_program::{clock::Clock, msg, program_error::ProgramError, sysvar::Sysvar};
