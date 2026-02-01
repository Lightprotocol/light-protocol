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

// Re-export everything from interface's v1 module
pub use light_sdk_interface::cpi::v1::*;

// LightCpiInstruction impl for LightSystemProgramCpi
mod invoke;
pub use invoke::LightSystemProgramCpi;
