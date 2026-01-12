//! High-level builders for ctoken operations.
//!
//!
//! ## Account Creation
//!
//! - [`CreateAssociatedTokenAccount`] - Create associated ctoken account (ATA) instruction
//! - [`CreateAssociatedTokenAccountCpi`] - Create associated ctoken account (ATA) via CPI
//! - [`CreateTokenAccount`] - Create ctoken account instruction
//! - [`CreateTokenAccountCpi`] - Create ctoken account via CPI
//!
//! ## Transfers
//!
//! - [`TransferInterfaceCpi`] - Transfer via CPI, auto-detect source/destination account types
//!
//! ## Decompress
//!
//! - [`DecompressToToken`] - Decompress compressed tokens to a cToken account
//!
//! ## Close
//!
//! - [`CloseTokenAccount`] - Create close ctoken account instruction
//! - [`CloseTokenAccountCpi`] - Close ctoken account via CPI
//!
//!
//! ## Mint
//!
//! - [`CreateCMint`] - Create cMint
//! - [`MintToToken`] - Mint tokens to ctoken accounts
//!
//! # Example: Create cToken Account Instruction
//!
//! ```rust
//! # use solana_pubkey::Pubkey;
//! use light_token_sdk::token::CreateAssociatedTokenAccount;
//! # let payer = Pubkey::new_unique();
//! # let owner = Pubkey::new_unique();
//! # let mint = Pubkey::new_unique();
//!
//! let instruction = CreateAssociatedTokenAccount::new(payer, owner, mint)
//!     .idempotent()
//!     .instruction()?;
//! # Ok::<(), solana_program_error::ProgramError>(())
//! ```
//!
//! # Example: Create cToken Account CPI
//!
//! ```rust,ignore
//! use light_token_sdk::token::{CreateAssociatedTokenAccountCpi, CompressibleParamsCpi};
//!
//! CreateAssociatedTokenAccountCpi {
//!     owner: ctx.accounts.owner.to_account_info(),
//!     mint: ctx.accounts.mint.to_account_info(),
//!     payer: ctx.accounts.payer.to_account_info(),
//!     associated_token_account: ctx.accounts.ctoken_account.to_account_info(),
//!     system_program: ctx.accounts.system_program.to_account_info(),
//!     bump,
//!     compressible: Some(CompressibleParamsCpi::default_with_accounts(
//!         ctx.accounts.compressible_config.to_account_info(),
//!         ctx.accounts.rent_sponsor.to_account_info(),
//!         ctx.accounts.system_program.to_account_info(),
//!     )),
//!     idempotent: true,
//! }
//! .invoke()?;
//! ```
//!

mod approve;
mod approve_checked;
mod burn;
mod burn_checked;
mod close;
mod compressible;
mod create;
mod create_ata;
mod create_cmint;
mod decompress;
mod decompress_cmint;
mod freeze;
mod mint_to;
mod revoke;
mod thaw;
mod token_mint_to;
mod token_mint_to_checked;
mod transfer_interface;
mod transfer_spl_to_token;
mod transfer_token;
mod transfer_token_checked;
mod transfer_token_to_spl;

pub use approve::*;
pub use approve_checked::*;
pub use burn::*;
pub use burn_checked::*;
pub use close::*;
pub use compressible::{CompressibleParams, CompressibleParamsCpi};
pub use create::*;
pub use create_ata::*;
pub use create_cmint::*;
pub use decompress::DecompressToToken;
pub use decompress_cmint::*;
pub use freeze::*;
use light_compressible::config::CompressibleConfig;
pub use light_token_interface::{
    instructions::{
        extensions::{CompressToPubkey, ExtensionInstructionData},
        mint_action::CompressedMintWithContext,
    },
    state::{Token, TokenDataVersion},
};
use light_token_types::POOL_SEED;
pub use mint_to::*;
pub use revoke::*;
use solana_account_info::AccountInfo;
use solana_pubkey::{pubkey, Pubkey};
pub use thaw::*;
pub use token_mint_to::*;
pub use token_mint_to_checked::*;
pub use transfer_interface::{SplInterface, TransferInterfaceCpi};
pub use transfer_spl_to_token::{TransferSplToToken, TransferSplToTokenCpi};
pub use transfer_token::*;
pub use transfer_token_checked::*;
pub use transfer_token_to_spl::{TransferTokenToSpl, TransferTokenToSplCpi};

/// System accounts required for CPI operations to Light Protocol.
///
/// Pass these accounts when invoking compressed token operations from your program.
///
/// # Fields
///
/// - `light_system_program` - Light System Program
/// - `cpi_authority_pda` - CPI authority (signs for your program)
/// - `registered_program_pda` - Your program's registration
/// - `account_compression_authority` - Compression authority
/// - `account_compression_program` - Account Compression Program
/// - `system_program` - Solana System Program
pub struct SystemAccountInfos<'info> {
    pub light_system_program: AccountInfo<'info>,
    pub cpi_authority_pda: AccountInfo<'info>,
    pub registered_program_pda: AccountInfo<'info>,
    pub account_compression_authority: AccountInfo<'info>,
    pub account_compression_program: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
}

/// System accounts with Pubkey references for instruction building.
///
/// ```rust
/// # use light_token_sdk::token::SystemAccounts;
/// # use solana_instruction::AccountMeta;
/// let system_accounts = SystemAccounts::default();
/// let accounts = vec![
///     AccountMeta::new_readonly(system_accounts.light_system_program, false),
///     AccountMeta::new_readonly(system_accounts.cpi_authority_pda, false),
///     // ...
/// ];
/// ```
#[derive(Debug, Clone, Copy)]
pub struct SystemAccounts {
    pub light_system_program: Pubkey,
    pub cpi_authority_pda: Pubkey,
    pub registered_program_pda: Pubkey,
    pub account_compression_authority: Pubkey,
    pub account_compression_program: Pubkey,
    pub system_program: Pubkey,
}

impl Default for SystemAccounts {
    fn default() -> Self {
        use crate::utils::CTokenDefaultAccounts;
        let defaults = CTokenDefaultAccounts::default();
        Self {
            light_system_program: defaults.light_system_program,
            cpi_authority_pda: defaults.cpi_authority_pda,
            registered_program_pda: defaults.registered_program_pda,
            account_compression_authority: defaults.account_compression_authority,
            account_compression_program: defaults.account_compression_program,
            system_program: defaults.system_program,
        }
    }
}

/// Compressed Token Program ID: `cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m`
pub const LIGHT_TOKEN_PROGRAM_ID: Pubkey = pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

pub const LIGHT_TOKEN_CPI_AUTHORITY: Pubkey =
    pubkey!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy");

pub const COMPRESSIBLE_CONFIG_V1: Pubkey = pubkey!("ACXg8a7VaqecBWrSbdu73W4Pg9gsqXJ3EXAqkHyhvVXg");

pub const RENT_SPONSOR: Pubkey = pubkey!("r18WwUxfG8kQ69bQPAB2jV6zGNKy3GosFGctjQoV4ti");

/// Returns the program ID for the Compressed Token Program
pub fn id() -> Pubkey {
    LIGHT_TOKEN_PROGRAM_ID
}

/// Return the cpi authority pda of the Compressed Token Program.
pub fn cpi_authority() -> Pubkey {
    LIGHT_TOKEN_CPI_AUTHORITY
}

pub fn get_spl_interface_pda_and_bump(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[POOL_SEED, mint.as_ref()], &LIGHT_TOKEN_PROGRAM_ID)
}

/// Returns the associated token address for a given owner and mint.
pub fn get_associated_token_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    get_associated_token_address_and_bump(owner, mint).0
}

/// Returns the associated token address and bump for a given owner and mint.
pub fn get_associated_token_address_and_bump(owner: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[&owner.to_bytes(), &id().to_bytes(), &mint.to_bytes()],
        &id(),
    )
}

/// Returns the default compressible config PDA.
pub fn config_pda() -> Pubkey {
    COMPRESSIBLE_CONFIG_V1
}

/// Returns the default rent sponsor PDA.
pub fn rent_sponsor_pda() -> Pubkey {
    RENT_SPONSOR
}

pub fn compression_authority_pda() -> Pubkey {
    CompressibleConfig::ctoken_v1_compression_authority_pda()
}

// Backwards compatibility aliases for old type names
pub use close::CloseTokenAccount as CloseCTokenAccount;
pub use create::CreateTokenAccount as CreateCTokenAccount;
pub use create_ata::{
    derive_token_ata as derive_ctoken_ata,
    CreateAssociatedTokenAccount as CreateAssociatedCTokenAccount,
};
pub use decompress::DecompressToToken as DecompressToCtoken;
pub use mint_to::MintToToken as MintToCToken;
pub use transfer_token::TransferToken as TransferCToken;
