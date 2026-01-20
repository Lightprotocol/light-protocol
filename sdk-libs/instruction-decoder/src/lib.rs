//! Instruction decoder library for Light Protocol.
//!
//! This crate provides:
//! - Core types for instruction decoding (DecodedField, DecodedInstruction, InstructionDecoder trait)
//! - Decoder registry for managing multiple program decoders
//! - Built-in decoders for Light Protocol programs (System, Compressed Token, etc.)
//! - Transaction logging configuration and formatting utilities
//!
//! The crate is designed to be independent of LiteSVM/test infrastructure,
//! enabling use in both test environments and standalone tools.
//!
//! Note: Most functionality is only available off-chain (not on Solana targets).

// Re-export solana types for use by dependent crates (available on all targets)
// Re-export derive macro for #[instruction_decoder]
pub use light_instruction_decoder_derive::instruction_decoder;
pub use solana_instruction;
pub use solana_pubkey;
pub use solana_signature;

// Core types available on all targets (needed by derive macros)
mod core;
pub use core::{DecodedField, DecodedInstruction, InstructionDecoder};

// Off-chain only modules (uses tabled, derive macros, DecoderRegistry)
#[cfg(not(target_os = "solana"))]
pub mod config;
#[cfg(not(target_os = "solana"))]
pub mod formatter;
#[cfg(not(target_os = "solana"))]
pub mod programs;
#[cfg(not(target_os = "solana"))]
pub mod registry;
#[cfg(not(target_os = "solana"))]
pub mod types;

// Re-export main types from types module
// Re-export config types
#[cfg(not(target_os = "solana"))]
pub use config::{EnhancedLoggingConfig, LogVerbosity};
// Re-export formatter
#[cfg(not(target_os = "solana"))]
pub use formatter::{Colors, TransactionFormatter};
// Re-export Light Protocol program decoders (requires light-protocol feature)
#[cfg(all(not(target_os = "solana"), feature = "light-protocol"))]
pub use programs::{
    AccountCompressionInstructionDecoder, CTokenInstructionDecoder, LightSystemInstructionDecoder,
    RegistryInstructionDecoder,
};
// Re-export program decoders (generic Solana programs)
#[cfg(not(target_os = "solana"))]
pub use programs::{
    ComputeBudgetInstructionDecoder, SplTokenInstructionDecoder, SystemInstructionDecoder,
    Token2022InstructionDecoder,
};
// Re-export registry
#[cfg(not(target_os = "solana"))]
pub use registry::DecoderRegistry;
#[cfg(not(target_os = "solana"))]
pub use types::{
    AccountAccess, AccountChange, CompressedAccountInfo, EnhancedInstructionLog,
    EnhancedTransactionLog, LightProtocolEvent, MerkleTreeChange, TransactionStatus,
};
