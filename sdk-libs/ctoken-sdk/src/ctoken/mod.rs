//! High-level builders for compressed token operations.
//!
//!
//! ## Account Creation
//!
//! - [`CreateAssociatedTokenAccount`] - Create associated ctoken account (ATA) instruction
//! - [`CreateAssociatedTokenAccountCpi`] - Create associated ctoken account (ATA) via CPI
//! - [`CreateCTokenAccount`] - Create ctoken account instruction
//! - [`CreateCTokenAccountCpi`] - Create ctoken account via CPI
//!
//! ## Transfers
//!
//! - [`TransferInterface`] - Transfer via CPI, auto-detect source/destination account types
//!
//! ## Close
//!
//! - [`CloseCTokenAccount`] - Create close ctoken account instruction
//! - [`CloseCTokenAccountCpi`] - Close ctoken account via CPI
//!
//!
//! ## Mint
//!
//! - [`CreateCMint`] - Create compressed mint
//! - [`MintToCToken`] - Mint tokens to ctoken accounts
//!
//! # Example: Create cToken Account Instruction
//!
//! ```rust
//! use light_ctoken_sdk::ctoken::CreateAssociatedTokenAccount;
//!
//! let instruction = CreateAssociatedTokenAccount::new(payer, owner, mint)
//!     .idempotent()
//!     .instruction()?;
//! ```
//!
//! # Example: Create cToken Account (CPI)
//!
//! ```rust,ignore
//! use light_ctoken_sdk::ctoken::{CreateAssociatedTokenAccountCpi, CompressibleParamsCpi};
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

mod close;
mod compressible;
mod create;
mod create_ata;
mod create_cmint;
mod mint_to;
mod transfer_ctoken;
mod transfer_ctoken_spl;
mod transfer_interface;
mod transfer_spl_ctoken;

pub use close::*;
pub use compressible::{CompressibleParams, CompressibleParamsCpi};
pub use create::*;
pub use create_ata::*;
pub use create_cmint::*;
use light_compressed_token_types::POOL_SEED;
use light_compressible::config::CompressibleConfig;
pub use light_ctoken_interface::{
    instructions::extensions::{compressible::CompressToPubkey, ExtensionInstructionData},
    state::TokenDataVersion,
};
pub use mint_to::*;
use solana_account_info::AccountInfo;
use solana_pubkey::{pubkey, Pubkey};
pub use transfer_ctoken::*;
pub use transfer_ctoken_spl::{TransferCtokenToSpl, TransferCtokenToSplCpi};
pub use transfer_interface::{SplInterface, TransferInterface};
pub use transfer_spl_ctoken::{TransferSplToCtoken, TransferSplToCtokenCpi};

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
/// # use light_ctoken_sdk::ctoken::SystemAccounts;
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
pub const CTOKEN_PROGRAM_ID: Pubkey = pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

pub const CTOKEN_CPI_AUTHORITY: Pubkey = pubkey!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy");

pub const COMPRESSIBLE_CONFIG_V1: Pubkey = pubkey!("ACXg8a7VaqecBWrSbdu73W4Pg9gsqXJ3EXAqkHyhvVXg");

pub const RENT_SPONSOR: Pubkey = pubkey!("r18WwUxfG8kQ69bQPAB2jV6zGNKy3GosFGctjQoV4ti");

/// Returns the program ID for the Compressed Token Program
pub fn id() -> Pubkey {
    CTOKEN_PROGRAM_ID
}

/// Return the cpi authority pda of the Compressed Token Program.
pub fn cpi_authority() -> Pubkey {
    CTOKEN_CPI_AUTHORITY
}

pub fn get_token_pool_address_and_bump(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[POOL_SEED, mint.as_ref()], &CTOKEN_PROGRAM_ID)
}

/// Returns the associated ctoken address for a given owner and mint.
pub fn get_associated_ctoken_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    get_associated_ctoken_address_and_bump(owner, mint).0
}

/// Returns the associated ctoken address and bump for a given owner and mint.
pub fn get_associated_ctoken_address_and_bump(owner: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
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
