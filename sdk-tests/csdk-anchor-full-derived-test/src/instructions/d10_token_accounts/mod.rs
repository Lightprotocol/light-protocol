//! D10 Test: Token Account and ATA creation via macro
//!
//! Tests #[light_account(init, token, ...)] and #[light_account(init, associated_token, ...)]
//! macro code generation for creating compressed token accounts.
//!
//! These tests verify:
//! - Single vault creation with seeds (token account)
//! - Single ATA creation (associated token account)
//! - Multiple vaults in same instruction
//! - Token accounts with PDAs
//! - Token accounts with mints

pub mod single_ata;
pub mod single_vault;

pub use single_ata::*;
pub use single_vault::*;
