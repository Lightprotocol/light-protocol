pub mod account;
pub mod error;
pub mod instruction;

// Conditional anchor re-exports
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};

// Re-export all modules
pub use account::*;
pub use error::*;
pub use instruction::*;

// Re-export types from light-compressed-token-types
pub use light_compressed_token_types::*;
