//! Create compressed mint CPI builder for pinocchio.
//!
//! Re-exports the generic `CreateMintsCpi` from `light_sdk_types`
//! specialized for pinocchio's `AccountInfo`.

pub use light_sdk_types::interface::cpi::create_mints::{
    derive_mint_compressed_address, find_mint_address, CreateMintsCpi, CreateMintsInfraAccounts,
    CreateMintsParams, SingleMintParams, DEFAULT_RENT_PAYMENT, DEFAULT_WRITE_TOP_UP,
};
