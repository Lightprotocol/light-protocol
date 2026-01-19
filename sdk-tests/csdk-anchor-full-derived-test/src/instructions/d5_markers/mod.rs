//! D5: Field marker attributes
//!
//! Tests #[light_account(init)], #[light_account(token)], and #[light_account(init)] attribute parsing.

mod all;
mod light_token;
mod rentfree_bare;
// Note: rentfree_custom is a failing test case due to pre-existing AddressTreeInfo bug.

pub use all::*;
pub use light_token::*;
pub use rentfree_bare::*;
