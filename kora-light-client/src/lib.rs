//! # kora-light-client
//!
//! Standalone Light Protocol instruction builders for solana-sdk 3.0 consumers.
//!
//! This crate has **zero `light-*` dependencies**. All types are duplicated
//! locally with byte-identical Borsh serialization to the on-chain program.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use kora_light_client::{pda, program_ids, types};
//!
//! let ata = pda::get_associated_token_address(&owner, &mint);
//! ```

pub mod account_select;
pub mod create_ata;
pub mod decompress;
pub mod error;
pub mod load_ata;
pub mod pda;
pub mod program_ids;
pub mod transfer;
pub mod types;
pub mod unwrap;
pub mod wrap;

// Re-export key types and functions
pub use account_select::select_input_accounts;
pub use create_ata::{create_ata_idempotent_instruction, CreateAta};
pub use decompress::create_decompress_instruction;
pub use error::KoraLightError;
pub use load_ata::{create_load_ata_batches, LoadAtaInput, LoadBatch};
pub use pda::{get_associated_token_address, get_associated_token_address_and_bump};
pub use program_ids::{LIGHT_SYSTEM_PROGRAM_ID, LIGHT_TOKEN_PROGRAM_ID};
pub use transfer::{create_transfer2_instruction, create_transfer_checked_instruction};
pub use types::{
    CompressedProof, CompressedTokenAccountInput, CompressedTokenInstructionDataTransfer2,
    Compression, CompressionMode, MultiInputTokenDataWithContext, MultiTokenTransferOutputData,
    PackedMerkleContext, SplInterfaceInfo, ValidityProofWithContext,
};
pub use unwrap::create_unwrap_instruction;
pub use wrap::create_wrap_instruction;
