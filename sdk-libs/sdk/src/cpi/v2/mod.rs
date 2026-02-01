//! V2 CPI for Light system program - optimized for compressed PDAs.
//!
//! # Main Types
//!
//! - [`InstructionDataInvokeCpiWithReadOnly`] - CPI instruction with read-only account support
//! - [`InstructionDataInvokeCpiWithAccountInfo`] - CPI instruction with account info
//! - [`CpiAccounts`] - CPI accounts struct

// CpiAccounts from sdk-types (v2)
pub use light_sdk_types::cpi_accounts::v2::CpiAccounts as GenericCpiAccounts;
pub type CpiAccounts<'c, 'info> =
    GenericCpiAccounts<'c, solana_account_info::AccountInfo<'info>>;

// Instruction types from light-compressed-account
pub use light_compressed_account::instruction_data::{
    cpi_context::CompressedCpiContext,
    with_account_info::InstructionDataInvokeCpiWithAccountInfo,
    with_readonly::{InAccount, InstructionDataInvokeCpiWithReadOnly},
};

// LightCpiInstruction impls for v2 instruction types
mod invoke;
