//! Create CToken account CPI builder for pinocchio.
//!
//! Re-exports the generic `CreateTokenAccountCpi` from `light_sdk_types`
//! specialized for pinocchio's `AccountInfo`.

// TODO: add types with generics set so that we dont expose the generics
pub use light_sdk_types::interface::cpi::create_token_accounts::{
    CreateTokenAccountCpi, CreateTokenAccountRentFreeCpi,
};
