//! Accounts-level interface for #[derive(LightAccounts)].
//!
//! This module contains traits and functions for context struct handling,
//! validation, and initialization at the accounts struct level.

pub mod create_pda;
pub mod finalize;
pub mod init_compressed_account;
