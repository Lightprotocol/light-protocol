pub mod constants;
pub mod instruction;
pub mod token_data;

// Conditional anchor re-exports
#[cfg(feature = "anchor")]
pub use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
pub use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};

// Re-export everything at the crate root level
pub use constants::*;
pub use instruction::*;
pub use token_data::*;
