//! Zero-copy AccountLoader support for compressible PDAs.
//!
//! This module demonstrates using AccountLoader<'info, T> instead of Account<'info, T>
//! for compressible accounts. Zero-copy accounts use bytemuck Pod/Zeroable traits
//! for direct memory access without deserialization.
//!
//! Key differences from Borsh accounts:
//! - State struct: `#[repr(C)]` + `Pod + Zeroable` instead of `#[account]`
//! - Data access: `ctx.accounts.record.load_mut()?.field` instead of `ctx.accounts.record.field`
//! - On-chain layout: Fixed-size Pod layout vs Borsh serialized
//! - Hashing: Still uses `try_to_vec()` (AnchorSerialize) for consistency

pub mod accounts;
pub mod derived_accounts;
pub mod derived_state;
pub mod state;

pub use accounts::*;
pub use derived_accounts::{
    PackedZeroCopyRecordSeeds, PackedZeroCopyRecordVariant, ZeroCopyRecordSeeds,
    ZeroCopyRecordVariant,
};
pub use derived_state::PackedZeroCopyRecord;
pub use state::ZeroCopyRecord;
