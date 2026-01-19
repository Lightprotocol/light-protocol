//! D5: Field marker attributes
//!
//! Tests #[light_account(init)], #[rentfree_token], and #[light_mint] attribute parsing.

mod all;
mod rentfree_bare;
mod rentfree_token;
// Note: rentfree_custom is a failing test case due to pre-existing AddressTreeInfo bug.

pub use all::*;
pub use rentfree_bare::*;
pub use rentfree_token::*;
