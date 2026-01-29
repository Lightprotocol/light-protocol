//! Accounts-level interface for #[derive(LightAccounts)].
//!
//! This module contains traits and functions for context struct handling,
//! validation, and initialization at the accounts struct level.

#[cfg(feature = "v2")]
pub mod create_pda;
pub mod finalize;
pub mod init_compressed_account;
