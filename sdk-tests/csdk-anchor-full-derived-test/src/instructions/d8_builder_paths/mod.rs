//! D8: Builder code generation paths
//!
//! Tests different builder code generation scenarios:
//! - pda_only: Only #[rentfree] fields (no tokens)
//! - multi_rentfree: Multiple #[rentfree] fields
//! - all: Multiple #[rentfree] fields with different state types

mod all;
mod multi_rentfree;
mod pda_only;

pub use all::*;
pub use multi_rentfree::*;
pub use pda_only::*;
