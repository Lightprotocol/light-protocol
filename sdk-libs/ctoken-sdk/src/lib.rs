//! Crate Structure:
//! What do I need to say:
//! 1. what is it?
//! 2. what are compressible, what are compressed tokens, and what are cmints?
//! 3. what is on mainnet what is on devnet?
//! 4. Modules
//! 5. structure of ctoken instructions:
//!     - (use create ata as example)
//!     - create the Instruction in client
//!     - create the Instruction in program
//! 6. Quickstart:
//!     - create cmint, create 2 accounts, mint ctokens, transfer ctokens
//!     - create spl mint, create 1 spl account, mint spl tokens, create 1 ctoken account, transfer spl tokens to ctoken account
//!     - use transfer interface for the transfers
//!     - inconvenient because it requires light-client to send the transactions -> link to the examples in the program examples directory
//!     - what about decompression?
//!
//! What do I need to reexport from light-ctoken-interface?
//! - users shouldn't need to import anything from /home/ananas/dev/light-protocol/program-libs/ctoken-interfaces
//!
//! # Compressed Token SDK
//!
//! Client and program utilities for compressed tokens on Light Protocol.
//!
//!
//!
//! ## Quick Start
//!
//! ```rust
//! use light_ctoken_sdk::ctoken::{
//!     CreateAssociatedTokenAccount,
//!     TransferCtoken,
//!     get_associated_ctoken_address,
//! };
//!
//! // Derive ATA address
//! let ata = get_associated_ctoken_address(&owner, &mint);
//!
//! // Create ATA instruction
//! let create_ix = CreateAssociatedTokenAccount::new(payer, owner, mint)
//!     .instruction()?;
//!
//! // Transfer instruction
//! let transfer_ix = TransferCtoken {
//!     source,
//!     destination,
//!     amount: 1_000_000,
//!     authority: owner,
//!     max_top_up: None,
//! }.instruction()?;
//! ```
//!
//! ## Modules
//!
//! - [`ctoken`] - High-level builders for token operations (recommended)
//! - [`compressed_token`] - Low-level v1/v2 instruction builders
//! - [`compressible`] - Compressible account utilities
//!
//! ## Feature Flags
//!
//! | Feature | Description |
//! |---------|-------------|
//! | `anchor` | Anchor framework integration |
//! | `compressible` | Compressible token account support |
//! | `cpi-context` | CPI context for batched operations |
//!
//! ## Common Operations
//!
//! | Operation | Client Builder | CPI Helper |
//! |-----------|----------------|------------|
//! | Create ATA | [`ctoken::CreateAssociatedTokenAccount`] | [`ctoken::CreateAssociatedTokenAccountInfos`] |
//! | Transfer | [`ctoken::TransferCtoken`] | [`ctoken::TransferCtokenAccountInfos`] |
//! | Mint | [`ctoken::MintToCToken`] | [`ctoken::MintToCTokenInfos`] |
//! | Close | [`ctoken::CloseAccount`] | [`ctoken::CloseAccountInfos`] |

pub mod compressed_token;
pub mod compressible;
pub mod ctoken;

pub mod error;
pub mod pack;
pub mod spl_interface;
pub mod utils;

// Conditional anchor re-exports
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
#[cfg(feature = "compressible")]
pub use compressible::decompress_runtime::{process_decompress_tokens_runtime, CTokenSeedProvider};
pub use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
pub use light_compressed_token_types::*;
pub use pack::compat;
#[cfg(feature = "compressible")]
pub use pack::{Pack, Unpack};
pub use utils::{
    account_meta_from_account_info, is_ctoken_account, AccountInfoToCompress,
    PackedCompressedTokenDataWithContext,
};
