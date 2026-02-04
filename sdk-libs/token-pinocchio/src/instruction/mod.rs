//! CPI builders for Light Token operations with Pinocchio.
//!
//! This module provides CPI (Cross-Program Invocation) builders that use
//! Pinocchio types for efficient on-chain token operations.
//!
//! ## Account Creation
//!
//! - [`CreateTokenAccountCpi`] - Create Light Token account via CPI
//! - [`CreateTokenAtaCpi`] - Create associated Light Token account (ATA) via CPI
//!
//! ## Transfers
//!
//! - [`TransferCpi`] - Transfer between Light Token accounts
//! - [`TransferFromSplCpi`] - Transfer from SPL token account to Light Token account
//! - [`TransferToSplCpi`] - Transfer from Light Token account to SPL token account
//! - [`TransferInterfaceCpi`] - Transfer via CPI, auto-detect source/destination account types
//!
//! ## Mint Operations
//!
//! - [`CreateMintCpi`] - Create single Light Mint via CPI
//! - [`CreateMintsCpi`] - Batch create Light Mints via CPI
//! - [`MintToCpi`] - Mint tokens to Light Token accounts
//!
//! ## Other Operations
//!
//! - [`ApproveCpi`] - Approve delegation
//! - [`RevokeCpi`] - Revoke delegation
//! - [`FreezeCpi`] - Freeze account
//! - [`ThawCpi`] - Thaw frozen account
//! - [`BurnCpi`] - Burn tokens
//! - [`CloseAccountCpi`] - Close Light Token account

mod approve;
mod burn;
mod burn_checked;
mod close;
mod create;
mod create_ata;
mod create_mint;
mod create_mints;
mod freeze;
mod mint_to;
mod mint_to_checked;
mod revoke;
mod thaw;
mod transfer;
mod transfer_checked;
mod transfer_from_spl;
mod transfer_interface;
mod transfer_to_spl;

pub use approve::*;
pub use burn::*;
pub use burn_checked::*;
pub use close::*;
pub use create::*;
pub use create_ata::{
    derive_associated_token_account, CreateTokenAtaCpi, CreateTokenAtaCpiIdempotent,
    CreateTokenAtaRentFreeCpi,
};
pub use create_mint::*;
pub use create_mints::{
    derive_mint_compressed_address as derive_mint_compressed_address_batch,
    find_mint_address as find_mint_address_batch, get_output_queue_next_index, CreateMints,
    CreateMintsCpi, CreateMintsParams, CreateMintsStaticAccounts, SingleMintParams,
    DEFAULT_RENT_PAYMENT, DEFAULT_WRITE_TOP_UP,
};
pub use freeze::*;
pub use light_compressed_account::instruction_data::compressed_proof::{
    CompressedProof, ValidityProof,
};
pub use light_token_interface::{
    instructions::{
        extensions::{CompressToPubkey, ExtensionInstructionData, TokenMetadataInstructionData},
        mint_action::MintWithContext,
    },
    state::{AdditionalMetadata, Token},
};
pub use mint_to::*;
pub use mint_to_checked::*;
use pinocchio::account_info::AccountInfo;
pub use revoke::*;
pub use thaw::*;
pub use transfer::*;
pub use transfer_checked::*;
pub use transfer_from_spl::TransferFromSplCpi;
pub use transfer_interface::{SplInterfaceCpi, TransferInterfaceCpi};
pub use transfer_to_spl::TransferToSplCpi;
/// System accounts required for CPI operations to Light Protocol.
///
/// Pass these accounts when invoking Light Token operations from your program.
///
/// # Fields
///
/// - `light_system_program` - Light System Program
/// - `cpi_authority_pda` - CPI authority (signs for your program)
/// - `registered_program_pda` - Your program's registration
/// - `account_compression_authority` - Compression authority
/// - `account_compression_program` - Account Compression Program
/// - `system_program` - Solana System Program
#[derive(Clone)]
pub struct SystemAccountInfos<'info> {
    pub light_system_program: &'info AccountInfo,
    pub cpi_authority_pda: &'info AccountInfo,
    pub registered_program_pda: &'info AccountInfo,
    pub account_compression_authority: &'info AccountInfo,
    pub account_compression_program: &'info AccountInfo,
    pub system_program: &'info AccountInfo,
}
