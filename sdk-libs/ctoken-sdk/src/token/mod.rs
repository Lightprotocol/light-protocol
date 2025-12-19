//! High-level builders for token operations.
//!
//!
//! ## Account Creation
//!
//! - [`CreateAssociatedTokenAccount`] - Create associated token account (ATA) instruction
//! - [`CreateAssociatedTokenAccountCpi`] - Create associated token account (ATA) via CPI
//! - [`CreateTokenAccount`] - Create token account instruction
//! - [`CreateTokenAccountCpi`] - Create token account via CPI
//!
//! ## Transfers
//!
//! - [`TransferInterfaceCpi`] - Transfer via CPI, auto-detect source/destination account types
//!
//! ## Decompress
//!
//! - [`Decompress`] - Decompress compressed tokens to a token account
//!
//! ## Close
//!
//! - [`CloseAccount`] - Create close token account instruction
//! - [`CloseAccountCpi`] - Close token account via CPI
//!
//!
//! ## Mint
//!
//! - [`CreateCMint`] - Create cMint
//! - [`MintTo`] - Mint tokens to token accounts
//!
//! # Example: Create Token Account Instruction
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
//! # Example: Create Token Account CPI
//!
//! ```rust,ignore
//! use light_token_sdk::token::{CreateAssociatedTokenAccountCpi, CompressibleParamsCpi};
//!
//! CreateAssociatedTokenAccountCpi {
//!     owner: ctx.accounts.owner.to_account_info(),
//!     mint: ctx.accounts.mint.to_account_info(),
//!     payer: ctx.accounts.payer.to_account_info(),
//!     associated_token_account: ctx.accounts.token_account.to_account_info(),
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
mod decompress;
mod mint_to;
mod transfer;
mod transfer_from_spl;
mod transfer_interface;
mod transfer_to_spl;

pub use close::*;
pub use compressible::{CompressibleParams, CompressibleParamsCpi};
pub use create::*;
pub use create_ata::*;
pub use create_cmint::*;
pub use decompress::Decompress;
use light_compressible::config::CompressibleConfig;
use light_ctoken_types::POOL_SEED;
pub use light_token_interface::{
    instructions::{
        extensions::{compressible::CompressToPubkey, ExtensionInstructionData},
        mint_action::CompressedMintWithContext,
    },
    state::{Token, TokenDataVersion},
};
pub use mint_to::*;
use solana_account_info::AccountInfo;
use solana_pubkey::{pubkey, Pubkey};
pub use transfer::*;
pub use transfer_from_spl::{TransferSplToLightToken, TransferSplToLightTokenCpi};
pub use transfer_interface::{SplInterface, TransferInterfaceCpi};
pub use transfer_to_spl::{TransferToSpl, TransferToSplCpi};

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
        use crate::utils::TokenDefaultAccounts;
        let defaults = TokenDefaultAccounts::default();
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

pub fn get_spl_interface_pda_and_bump(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[POOL_SEED, mint.as_ref()], &CTOKEN_PROGRAM_ID)
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
