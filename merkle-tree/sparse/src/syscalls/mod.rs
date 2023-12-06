//! This module is a partial copy from
//! [solana-program](https://github.com/solana-labs/solana/blob/master/sdk/program/src/syscalls/definitions.rs),
//! which is licensed under Apache License 2.0.
//!
//! The purpose of the module is to provide definition of Poseidon syscall
//! without upgrading solana-program and Anchor just yet.

#[cfg(target_os = "solana")]
mod definitions;

#[cfg(target_os = "solana")]
pub use definitions::*;
