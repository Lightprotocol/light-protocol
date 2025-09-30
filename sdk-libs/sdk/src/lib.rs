//!
//! # Core Functionality
//! 1. Instruction
//!     - `AccountMeta` - Compressed account metadata structs for instruction data.
//!     - `PackedAccounts` - Abstraction to prepare accounts offchain for instructions with compressed accounts.
//! 2. Program logic
//!     - `LightAccount` - Compressed account abstraction similar to anchor Account.
//!     - `derive_address` - Create a compressed account address.
//!     - `LightHasher` - DeriveMacro to derive a hashing scheme from a struct layout.
//!     - `LightDiscriminator` - DeriveMacro to derive a compressed account discriminator.
//! 3. Cpi
//!     - `CpiAccounts` - Prepare accounts to cpi the light system program.
//!     - `LightSystemProgramCpi` - Prepare instruction data to cpi the light system program.
//!     - `LightSystemProgramCpi::invoke` - Invoke the light system program via cpi.
//!
//!
//! # Features
//! 1. `anchor` - Derives AnchorSerialize, AnchorDeserialize instead of BorshSerialize, BorshDeserialize.
//!
//! 2. `v2` - light protocol program v2 are currently in audit and only available on local host and with light-program-test.
//!    Deploy on devnet and mainnet only without v2 features enabled.
//!
//! ### Example Solana program code to create a compressed account
//! ```rust, compile_fail
//! use anchor_lang::{prelude::*, Discriminator};
//! use light_sdk::{
//!     account::LightAccount,
//!     address::v1::derive_address,
//!     cpi::{v1::LightSystemProgramCpi, CpiAccounts, InvokeLightSystemProgram, LightCpiInstruction},
//!     derive_light_cpi_signer,
//!     instruction::{account_meta::CompressedAccountMeta, PackedAddressTreeInfo},
//!     CpiSigner, LightDiscriminator, LightHasher, ValidityProof,
//! };
//!
//! declare_id!("2tzfijPBGbrR5PboyFUFKzfEoLTwdDSHUjANCw929wyt");
//!
//! pub const LIGHT_CPI_SIGNER: CpiSigner =
//!     derive_light_cpi_signer!("2tzfijPBGbrR5PboyFUFKzfEoLTwdDSHUjANCw929wyt");
//!
//! #[program]
//! pub mod counter {
//!
//!     use super::*;
//!
//!     pub fn create_compressed_account<'info>(
//!         ctx: Context<'_, '_, '_, 'info, CreateCompressedAccount<'info>>,
//!         proof: ValidityProof,
//!         address_tree_info: PackedAddressTreeInfo,
//!         output_tree_index: u8,
//!     ) -> Result<()> {
//!         let light_cpi_accounts = CpiAccounts::new(
//!             ctx.accounts.fee_payer.as_ref(),
//!             ctx.remaining_accounts,
//!             crate::LIGHT_CPI_SIGNER,
//!         )?;
//!
//!         let (address, address_seed) = derive_address(
//!             &[b"counter", ctx.accounts.fee_payer.key().as_ref()],
//!             &address_tree_info.get_tree_pubkey(&light_cpi_accounts)?,
//!             &crate::ID,
//!         );
//!         let new_address_params = address_tree_info
//!             .into_new_address_params_packed(address_seed);
//!
//!         let mut my_compressed_account = LightAccount::<'_, CounterAccount>::new_init(
//!             &crate::ID,
//!             Some(address),
//!             output_tree_index,
//!         );
//!
//!         my_compressed_account.owner = ctx.accounts.fee_payer.key();
//!
//!         let cpi_instruction = LightSystemProgramCpi::new_cpi(crate::LIGHT_CPI_SIGNER, proof)
//!             .with_light_account(my_compressed_account)?
//!             .with_new_addresses(&[new_address_params]);
//!
//!         cpi_instruction
//!             .invoke(light_cpi_accounts)?;
//!
//!         Ok(())
//!     }
//! }
//!
//! #[derive(Accounts)]
//! pub struct CreateCompressedAccount<'info> {
//!    #[account(mut)]
//!    pub fee_payer: Signer<'info>,
//! }
//!
//! #[derive(Clone, Debug, Default, LightDiscriminator)]
//!pub struct CounterAccount {
//!    pub owner: Pubkey,
//!    pub counter: u64
//!}
//! ```

/// Compressed account abstraction similar to anchor Account.
pub mod account;
pub use account::sha::LightAccount;

/// SHA256-based variants
pub mod sha {
    pub use light_sdk_macros::LightHasherSha as LightHasher;

    pub use crate::account::sha::LightAccount;
}

/// Functions to derive compressed account addresses.
pub mod address;
/// Utilities to invoke the light-system-program via cpi.
pub mod cpi;
pub mod error;
/// Utilities to build instructions for programs with compressed accounts.
pub mod instruction;
pub mod legacy;
pub mod token;
/// Transfer compressed sol between compressed accounts.
pub mod transfer;
pub mod utils;

#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use light_account_checks::{self, discriminator::Discriminator as LightDiscriminator};
pub use light_hasher;
use light_hasher::DataHasher;
pub use light_sdk_macros::{
    derive_light_cpi_signer, light_system_accounts, LightDiscriminator, LightHasher,
    LightHasherSha, LightTraits,
};
pub use light_sdk_types::constants;
use solana_account_info::AccountInfo;
use solana_cpi::invoke_signed;
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;
