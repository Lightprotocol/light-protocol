//! D6: Account type extraction
//!
//! Tests macro handling of different account wrapper types:
//! - Account<'info, T> - direct extraction
//! - Box<Account<'info, T>> - Box unwrap with is_boxed = true

mod account;
mod all;
mod boxed;

pub use account::*;
pub use all::*;
pub use boxed::*;
