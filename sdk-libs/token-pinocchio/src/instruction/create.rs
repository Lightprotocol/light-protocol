//! Create CToken account CPI builder for pinocchio.
//!
//! Re-exports the generic `CreateTokenAccountCpi` from `light_sdk_types`
//! specialized for pinocchio's `AccountInfo`.

pub use light_sdk_types::interface::cpi::create_token_accounts::{
    CreateTokenAccountCpi, CreateTokenAccountRentFreeCpi,
};
