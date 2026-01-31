//! V2 CPI for Light system program - optimized for compressed PDAs.
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

// Re-export everything from interface's v2 module
pub use light_sdk_interface::cpi::v2::*;

// SDK extension: WithLightAccount impls for v2 instruction types
mod invoke;
