//! # Overview
//!
//! This module provides types and utilities for building Solana instructions that work with
//! compressed accounts. The main workflow involves:
//! ```text
//!  |- Client
//!  |  |- Get ValidityProof from RPC.
//!  |  |- pack accounts with PackedAccounts into PackedAddressTreeInfo and PackedStateTreeInfo.
//!  |  |- pack CompressedAccountMeta.
//!  |  |- Build Instruction from packed accounts and CompressedAccountMetas.
//!  |  |_ Send transaction
//!  |
//!  |_ Custom Program
//!     |- use PackedAddressTreeInfo to create a new address.
//!     |- use CompressedAccountMeta to instantiate a LightAccount struct.
//!     |
//!     |_ Light System Program CPI
//! ```
//! ## Main Types
//!
//! - [`PackedAddressTreeInfo`](crate::instruction::PackedAddressTreeInfo) - Indices of address tree and queue accounts.
//! - [`PackedStateTreeInfo`](crate::instruction::PackedStateTreeInfo) - Indices of state tree and queue accounts.
//! - [`PackedAccounts`](crate::instruction::PackedAccounts) - Packs accounts and creates indices for instruction building (client-side).
//! - [`SystemAccountMetaConfig`](crate::instruction::SystemAccountMetaConfig) - Configures which Light system program accounts to add to [`PackedAccounts`](crate::instruction::PackedAccounts).
//! - [`ValidityProof`](crate::instruction::ValidityProof) - Proves that new addresses don't exist yet, and compressed account state exists.
//! - [`CompressedAccountMeta`](crate::instruction::account_meta::CompressedAccountMeta) - Metadata for compressed accounts.
//!
//! ## Compressed Account Metas
//! Instruction data types to pass compressed account metadata into instructions.
//! [`CompressedAccountMeta`](crate::instruction::account_meta::CompressedAccountMeta) and variations with and without lamports and addresses are used to instantiate LightAccount structs in your program.
//!
//! ## Packed Structs Pattern
//!
//! Structs prefixed with `Packed` (eg [`PackedAddressTreeInfo`](crate::instruction::PackedAddressTreeInfo), [`PackedStateTreeInfo`](crate::instruction::PackedStateTreeInfo)) are instruction data
//! structs that contain account **indices** instead of **pubkeys** to reduce instruction size.
//!
//! - `Packed*` structs: Contain indices (u8) for use in instruction data.
//! - Non-`Packed` structs: Contain pubkeys (Pubkey) for use in the client, and are returned by RPC methods.
//! - [`PackedAccounts`](crate::instruction::PackedAccounts): Manages account deduplication and index assignment to create `Packed*` structs.

// TODO: link to examples

// Re-export instruction types from sdk-types (available on all targets)
// SDK-specific: ValidityProof and CompressedProof
pub use light_compressed_account::instruction_data::compressed_proof::{
    CompressedProof, ValidityProof,
};
pub use light_sdk_types::instruction::*;
// Re-export pack_accounts utilities (off-chain only, requires std for HashMap)
#[cfg(not(target_os = "solana"))]
pub use light_sdk_types::pack_accounts::*;

// SDK-specific: system account helpers (depend on find_cpi_signer_macro!)
mod system_accounts;
pub use system_accounts::*;

// SDK-specific: tree info packing/unpacking
mod tree_info;
pub use tree_info::*;

// Newtype wrapper around generic PackedAccounts<AccountMeta> with inherent system account methods
#[cfg(not(target_os = "solana"))]
mod packed_accounts;
#[cfg(not(target_os = "solana"))]
pub use packed_accounts::PackedAccounts;
