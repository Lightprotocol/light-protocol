//! Two mints instruction - creates two compressed mints using derived PDAs.

pub mod accounts;
mod derived;

pub use accounts::*;
pub use derived::process_create_derived_mints;
