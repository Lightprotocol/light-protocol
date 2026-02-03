//! # Light Token Pinocchio SDK
//!
//! Pinocchio-based SDK for Light Token operations via CPI.
//!
//!
//! ## CPI Operations
//!
//! | Operation | CPI Builder |
//! |-----------|-------------|
//! | Transfer | [`TransferCpi`](instruction::TransferCpi) |
//! | Transfer Checked | [`TransferCheckedCpi`](instruction::TransferCheckedCpi) |
//! | Mint To | [`MintToCpi`](instruction::MintToCpi) |
//! | Mint To Checked | [`MintToCheckedCpi`](instruction::MintToCheckedCpi) |
//! | Burn | [`BurnCpi`](instruction::BurnCpi) |
//! | Burn Checked | [`BurnCheckedCpi`](instruction::BurnCheckedCpi) |
//! | Approve | [`ApproveCpi`](instruction::ApproveCpi) |
//! | Revoke | [`RevokeCpi`](instruction::RevokeCpi) |
//! | Freeze | [`FreezeCpi`](instruction::FreezeCpi) |
//! | Thaw | [`ThawCpi`](instruction::ThawCpi) |
//! | Close Account | [`CloseAccountCpi`](instruction::CloseAccountCpi) |
//! | Create Token Account | [`CreateTokenAccountCpi`](instruction::CreateTokenAccountCpi) |
//! | Create Token ATA | [`CreateTokenAtaCpi`](instruction::CreateTokenAtaCpi) |
//! | Create Mint | [`CreateMintCpi`](instruction::CreateMintCpi) |
//! | Create Mints (Batch) | [`CreateMintsCpi`](instruction::CreateMintsCpi) |
//! | Decompress Mint | [`DecompressMintCpi`](instruction::DecompressMintCpi) |
//!
//! ## Example: Transfer via CPI
//!
//! ```rust,ignore
//! use light_token_pinocchio::instruction::TransferCpi;
//!
//! TransferCpi {
//!     source: &ctx.accounts.source,
//!     destination: &ctx.accounts.destination,
//!     amount: 100,
//!     authority: &ctx.accounts.authority,
//!     system_program: &ctx.accounts.system_program,
//!     max_top_up: None,
//!     fee_payer: None,
//! }
//! .invoke()?;
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod constants;
pub mod error;
pub mod instruction;
