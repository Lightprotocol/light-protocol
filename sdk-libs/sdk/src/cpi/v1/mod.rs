//! V1 CPI for Light system program.
//!
//! # Main Types
//!
//! - [`LightSystemProgramCpi`] - CPI instruction data builder
//! - [`CpiAccounts`] - CPI accounts struct
//!
//!
//! # Advanced Usage
//!
//! For maximum flexible light system program CPIs, see the [`lowlevel`] module or use `light-compressed-account` directly.

mod accounts;
mod invoke;

pub use accounts::CpiAccounts;
pub use invoke::LightSystemProgramCpi;

/// Low-level types and functions for flexible Light system program CPIs.
///
/// # Main Types
///
/// For most use cases, you only need:
/// - [`LightSystemProgramCpi`] - Main CPI interface
/// - [`CpiAccounts`] - Account management
///
/// The remaining types in this module are exported for low-level operations and internal use.
pub mod lowlevel {
    pub use super::accounts::{
        get_account_metas_from_config, CpiInstructionConfig, SYSTEM_ACCOUNTS_LEN,
    };
}
