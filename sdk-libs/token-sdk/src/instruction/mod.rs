//! High-level builders for Light Token operations.
//!
//!
//! ## Account Creation
//!
//! - [`CreateAssociatedTokenAccount`] - Create associated Light Token account (ATA) instruction
//! - [`CreateTokenAtaCpi`] - Create associated Light Token account (ATA) via CPI
//! - [`CreateTokenAccount`] - Create Light Token account instruction
//! - [`CreateTokenAccountCpi`] - Create Light Token account via CPI
//!
//! ## Transfers
//!
//! - [`TransferInterfaceCpi`] - Transfer via CPI, auto-detect source/destination account types
//!
//! ## Decompress
//!
//! - [`Decompress`] - Decompress compressed tokens to a Light Token account
//!
//! ## Close
//!
//! - [`CloseAccount`] - Create close Light Token account instruction
//! - [`CloseAccountCpi`] - Close Light Token account via CPI
//!
//!
//! ## Mint
//!
//! - [`CreateMint`] - Create Light Mint
//! - [`create_mints`] - Create multiple Light Mints in a batch
//! - [`MintTo`] - Mint tokens to Light Token accounts
//!
//! ## Revoke and Thaw
//!
//! - [`Revoke`] - Revoke delegation for a Light Token account
//! - [`RevokeCpi`] - Revoke delegation via CPI
//! - [`Thaw`] - Thaw a frozen Light Token account
//! - [`ThawCpi`] - Thaw a frozen Light Token account via CPI
//!
//! # Example: Create Light Token Account Instruction
//!
//! ```rust
//! # use solana_pubkey::Pubkey;
//! use light_token::instruction::CreateAssociatedTokenAccount;
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
//! use light_token::instruction::CreateTokenAtaCpi;
//!
//! CreateTokenAtaCpi {
//!     payer: ctx.accounts.payer.to_account_info(),
//!     owner: ctx.accounts.owner.to_account_info(),
//!     mint: ctx.accounts.mint.to_account_info(),
//!     ata: ctx.accounts.user_ata.to_account_info(),
//!     bump,
//! }
//! .idempotent()
//! .rent_free(
//!     ctx.accounts.light_token_config.to_account_info(),
//!     ctx.accounts.rent_sponsor.to_account_info(),
//!     ctx.accounts.system_program.to_account_info(),
//! )
//! .invoke()?;
//! ```
//!
//! # Example: Create rent-free vault via CPI (with PDA signing)
//!
//! ```rust,ignore
//! use light_token::instruction::CreateTokenAccountCpi;
//!
//! CreateTokenAccountCpi {
//!     payer: ctx.accounts.payer.to_account_info(),
//!     account: ctx.accounts.vault.to_account_info(),
//!     mint: ctx.accounts.mint.to_account_info(),
//!     owner: ctx.accounts.vault_authority.key(),
//! }
//! .rent_free(
//!     ctx.accounts.light_token_config.to_account_info(),
//!     ctx.accounts.rent_sponsor.to_account_info(),
//!     ctx.accounts.system_program.to_account_info(),
//!     &crate::ID,
//! )
//! .invoke_signed(&[b"vault", mint.key().as_ref(), &[bump]])?;
//! ```
//!

mod approve;
mod burn;
mod burn_checked;
mod close;
mod compressible;
mod create;
mod create_ata;
mod create_mint;
mod create_mints;
// Decompress instruction builder is client-side only (uses PackedAccounts)
#[cfg(not(target_os = "solana"))]
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
pub use burn::*;
pub use burn_checked::*;
pub use close::{CloseAccount, CloseAccountCpi};
pub use compressible::{CompressibleParams, CompressibleParamsCpi};
pub use create::*;
pub use create_ata::{
    derive_associated_token_account, derive_associated_token_account as derive_token_ata,
    CreateAssociatedTokenAccount, CreateTokenAtaCpi as CreateAssociatedAccountCpi,
    CreateTokenAtaCpi,
};
pub use create_mint::*;
pub use create_mints::*;
#[cfg(not(target_os = "solana"))]
pub use decompress::Decompress;
pub use decompress_mint::*;
pub use freeze::*;
pub use light_token_interface::{
    instructions::{
        extensions::{CompressToPubkey, ExtensionInstructionData, TokenMetadataInstructionData},
        mint_action::MintWithContext,
    },
    state::{AdditionalMetadata, Token, TokenDataVersion},
};
pub use mint_to::{MintTo, MintToCpi};
pub use mint_to_checked::*;
pub use revoke::{Revoke, RevokeCpi};
use solana_account_info::AccountInfo;
use solana_pubkey::Pubkey;
pub use thaw::{Thaw, ThawCpi};
pub use transfer::*;
pub use transfer_checked::*;
pub use transfer_from_spl::{TransferFromSpl, TransferFromSplCpi};
pub use transfer_interface::{
    SplInterface, SplInterfaceCpi, TransferInterface, TransferInterfaceCpi,
};
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
/// # use light_token::instruction::SystemAccounts;
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
        use light_compressed_token_sdk::utils::TokenDefaultAccounts;
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

// Re-export constants for backwards compatibility
pub use crate::{
    constants::{
        compression_authority_pda, config_pda, rent_sponsor_pda, LIGHT_TOKEN_CONFIG,
        LIGHT_TOKEN_CPI_AUTHORITY, LIGHT_TOKEN_PROGRAM_ID, RENT_SPONSOR_V1 as RENT_SPONSOR,
    },
    cpi_authority, id,
    spl_interface::get_spl_interface_pda_and_bump,
    utils::{get_associated_token_address, get_associated_token_address_and_bump},
};
