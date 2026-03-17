//! # kora-light-client
//!
//! Standalone Light Protocol instruction builders for solana-sdk 3.0 consumers.
//!
//! This crate has **zero `light-*` dependencies**. All types are duplicated
//! locally with byte-identical Borsh serialization to the on-chain program.
//!
//! | Builder | Description |
//! |---------|-------------|
//! | [`CreateAta`] | Create an associated light-token account |
//! | [`Transfer2`] | Compressed-to-compressed token transfer |
//! | [`TransferChecked`] | Decompressed ATA-to-ATA transfer |
//! | [`Decompress`] | Decompress compressed tokens to on-chain account |
//! | [`Wrap`] | Wrap SPL/T22 tokens to light-token account |
//! | [`Unwrap`] | Unwrap light-token to SPL/T22 account |
//!
//! ## Utilities
//!
//! | Function | Description |
//! |----------|-------------|
//! | [`select_input_accounts`] | Greedy account selection (max 8) |
//! | [`create_load_ata_batches`] | Multi-transaction batch orchestration |
//! | [`get_associated_token_address`] | Derive light-token ATA address |
//! | [`find_spl_interface_pda`] | Derive SPL pool PDA |
//!
//! ## Usage
//!
//! ```rust,ignore
//! use kora_light_client::{Transfer2, get_associated_token_address};
//!
//! let ata = get_associated_token_address(&owner, &mint);
//!
//! let ix = Transfer2 {
//!     payer, authority, mint,
//!     inputs: &accounts,
//!     proof: &proof,
//!     destination_owner: recipient,
//!     amount: 1_000,
//! }.instruction()?;
//! ```

pub mod account_select;
pub mod create_ata;
pub mod decompress;
pub mod error;
pub mod load_ata;
mod packed_accounts;
pub mod pda;
pub mod program_ids;
pub mod transfer;
pub mod types;
pub mod unwrap;
pub mod wrap;

// Builder structs
pub use create_ata::CreateAta;
pub use decompress::Decompress;
pub use transfer::{Transfer2, TransferChecked};
pub use unwrap::Unwrap;
pub use wrap::Wrap;

// Consumer-facing types
pub use types::{
    CompressedProof, CompressedTokenAccountInput, SplInterfaceInfo, ValidityProofWithContext,
};

// Utilities
pub use account_select::select_input_accounts;
pub use error::KoraLightError;
pub use load_ata::{create_load_ata_batches, LoadAtaInput, LoadBatch, WrapSource};

// PDA helpers
pub use pda::{
    find_spl_interface_pda, find_spl_interface_pda_with_index, get_associated_token_address,
    get_associated_token_address_and_bump,
};

// Constants
pub use program_ids::{
    LIGHT_LUT_DEVNET, LIGHT_LUT_MAINNET, LIGHT_SYSTEM_PROGRAM_ID, LIGHT_TOKEN_PROGRAM_ID,
};
