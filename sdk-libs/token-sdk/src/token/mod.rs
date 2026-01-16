//! High-level builders for ctoken operations.
//!
//!
//! ## Account Creation
//!
//! - [`CreateAssociatedCTokenAccount`] - Create associated ctoken account (ATA) instruction
//! - [`CreateCTokenAtaCpi`] - Create associated ctoken account (ATA) via CPI
//! - [`CreateCTokenAccount`] - Create ctoken account instruction
//! - [`CreateTokenAccountCpi`] - Create ctoken account via CPI
//!
//! ## Transfers
//!
//! - [`TransferInterfaceCpi`] - Transfer via CPI, auto-detect source/destination account types
//!
//! ## Decompress
//!
//! - [`Decompress`] - Decompress compressed tokens to a cToken account
//!
//! ## Close
//!
//! - [`CloseTokenAccount`] - Create close ctoken account instruction
//! - [`CloseTokenAccountCpi`] - Close ctoken account via CPI
//!
//!
//! ## Mint
//!
//! - [`CreateMint`] - Create cMint
//! - [`MintTo`] - Mint tokens to ctoken accounts
//!
//! ## Revoke and Thaw
//!
//! - [`Revoke`] - Revoke delegation for a ctoken account
//! - [`RevokeCpi`] - Revoke delegation via CPI
//! - [`Thaw`] - Thaw a frozen ctoken account
//! - [`ThawCpi`] - Thaw a frozen ctoken account via CPI
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
//! # Example: Create rent-free ATA via CPI
//!
//! ```rust,ignore
//! use light_token_sdk::token::CreateCTokenAtaCpi;
//!
//! CreateCTokenAtaCpi {
//!     payer: ctx.accounts.payer.to_account_info(),
//!     owner: ctx.accounts.owner.to_account_info(),
//!     mint: ctx.accounts.mint.to_account_info(),
//!     ata: ctx.accounts.user_ata.to_account_info(),
//!     bump,
//! }
//! .idempotent()
//! .rent_free(
//!     ctx.accounts.ctoken_config.to_account_info(),
//!     ctx.accounts.rent_sponsor.to_account_info(),
//!     ctx.accounts.system_program.to_account_info(),
//! )
//! .invoke()?;
//! ```
//!
//! # Example: Create rent-free vault via CPI (with PDA signing)
//!
//! ```rust,ignore
//! use light_token_sdk::token::CreateTokenAccountCpi;
//!
//! CreateTokenAccountCpi {
//!     payer: ctx.accounts.payer.to_account_info(),
//!     account: ctx.accounts.vault.to_account_info(),
//!     mint: ctx.accounts.mint.to_account_info(),
//!     owner: ctx.accounts.vault_authority.key(),
//! }
//! .rent_free(
//!     ctx.accounts.ctoken_config.to_account_info(),
//!     ctx.accounts.rent_sponsor.to_account_info(),
//!     ctx.accounts.system_program.to_account_info(),
//!     &crate::ID,
//! )
//! .invoke_signed(&[b"vault", mint.key().as_ref(), &[bump]])?;
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
mod create_mint;
mod decompress;
mod decompress_mint;
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
pub use approve_checked::*;
pub use burn::*;
pub use burn_checked::*;
pub use close::{CloseAccount, CloseAccountCpi};
pub use compressible::{CompressibleParams, CompressibleParamsCpi};
pub use create::*;
pub use create_ata::CreateCTokenAtaCpi as CreateAssociatedAccountCpi;
pub use create_ata::{derive_token_ata, CreateAssociatedTokenAccount, CreateCTokenAtaCpi};
pub use create_mint::*;
pub use decompress::Decompress;
pub use decompress_mint::*;
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
pub use mint_to::{MintTo, MintToCpi};
pub use mint_to_checked::*;
pub use revoke::{Revoke, RevokeCpi};
use solana_account_info::AccountInfo;
use solana_pubkey::{pubkey, Pubkey};
pub use thaw::{Thaw, ThawCpi};
pub use transfer::*;
pub use transfer_checked::*;
pub use transfer_from_spl::{TransferFromSpl, TransferFromSplCpi};
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
#[derive(Clone)]
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
    CompressibleConfig::light_token_v1_compression_authority_pda()
}
