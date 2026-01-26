//! Token account creation - manual implementation of macro-generated code.

pub mod accounts;
mod derived;

pub use accounts::*;
pub use derived::process_create_token_vault;
