//! # Overview
//!
//! This module provides types and utilities for building Solana instructions that work with
//! compressed accounts. The main workflow involves:
//! ```text
//!  ├─ 𝐂𝐥𝐢𝐞𝐧𝐭
//!  │  ├─ Get ValidityProof from RPC.
//!  │  ├─ pack accounts with PackedAccounts into PackedAddressTreeInfo and PackedStateTreeInfo.
//!  │  ├─ pack CompressedAccountMeta.
//!  │  ├─ Build Instruction from packed accounts and CompressedAccountMetas.
//!  │  └─ Send transaction
//!  │
//!  └─ 𝐂𝐮𝐬𝐭𝐨𝐦 𝐏𝐫𝐨𝐠𝐫𝐚𝐦
//!     ├─ use PackedAddressTreeInfo to create a new address.
//!     ├─ use CompressedAccountMeta to instantiate a LightAccount struct.
//!     │
//!     └─ 𝐋𝐢𝐠𝐡𝐭 𝐒𝐲𝐬𝐭𝐞𝐦 𝐏𝐫𝐨𝐠𝐫𝐚𝐦 𝐂𝐏𝐈
//!        ├─ Verify ValidityProof.
//!        ├─ Update State Merkle tree.
//!        ├─ Update Address Merkle tree.
//!        └─ Complete atomic state transition.
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

mod pack_accounts;
mod system_accounts;
mod tree_info;

/// Zero-knowledge proof to prove the validity of existing compressed accounts and new addresses.
pub use light_compressed_account::instruction_data::compressed_proof::{
    CompressedProof, ValidityProof,
};
pub use light_sdk_types::instruction::*;
pub use pack_accounts::*;
pub use system_accounts::*;
pub use tree_info::*;
