//! D11: Zero-copy (Pod) state structs for AccountLoader tests.
//!
//! These structs use:
//! - `#[account(zero_copy)]` for Pod serialization
//! - `#[repr(C)]` for predictable memory layout
//! - `CompressionInfo` from light_sdk::interface (24 bytes, Pod-compatible)

pub mod basic;
pub mod with_params;
pub mod with_seeds;

pub use basic::*;
pub use with_params::*;
pub use with_seeds::*;
