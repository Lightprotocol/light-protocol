//! D8: Builder code generation paths
//!
//! Tests different builder code generation scenarios:
//! - pda_only: Only #[light_account(init)] fields (no tokens)
//! - multi_rentfree: Multiple #[light_account(init)] fields
//! - all: Multiple #[light_account(init)] fields with different state types

mod all;
mod multi_rentfree;
mod pda_only;

pub use all::*;
pub use multi_rentfree::*;
pub use pda_only::*;
