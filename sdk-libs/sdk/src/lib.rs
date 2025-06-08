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
//!     - `CpiInputs` - Prepare instruction data to cpi the light system program.
//!     - `invoke_light_system_program` - Invoke the light system program via cpi.
//!
//!
//! # Features
//! 1. `anchor` - Derives AnchorSerialize, AnchorDeserialize instead of BorshSerialize, BorshDeserialize.
//!
//! 2. `v2` - light protocol program v2 are currently in audit and only available on local host and with light-program-test.
//!    Deploy on devnet and mainnet only without v2 features enabled.
//!
//! ### Example Solana program code to create a compressed account
//! ```ignore
//! use anchor_lang::{prelude::*, Discriminator};
//! use light_sdk::{
//!     account::LightAccount,
//!     address::v1::derive_address,
//!     cpi::{CpiAccounts, CpiInputs},
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
//!         )
//!         .map_err(ProgramError::from)?;
//!
//!         let (address, address_seed) = derive_address(
//!             &[b"counter", ctx.accounts.fee_payer.key().as_ref()],
//!             &light_cpi_accounts.tree_accounts()
//!                 [address_tree_info.address_merkle_tree_pubkey_index as usize]
//!                 .key(),
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
//!         let cpi_inputs = CpiInputs::new_with_address(
//!             proof,
//!             vec![my_compressed_account
//!                 .to_account_info()
//!                 .map_err(ProgramError::from)?],
//!             vec![new_address_params],
//!         );
//!
//!         cpi_inputs
//!             .invoke_light_system_program(light_cpi_accounts)
//!             .map_err(ProgramError::from)?;
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
//! #[derive(Clone, Debug, Default, LightHasher, LightDiscriminator)]
//!pub struct CounterAccount {
//!    #[hash]
//!    pub owner: Pubkey,
//!    pub counter: u64
//!}
//! ```

/// Compressed account abstraction similar to anchor Account.
pub mod account;
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
pub use light_sdk_macros::{
    derive_light_cpi_signer, light_system_accounts, LightDiscriminator, LightHasher, LightTraits,
};
pub use light_sdk_types::constants;
use solana_account_info::AccountInfo;
use solana_cpi::invoke_signed;
use solana_instruction::{AccountMeta, Instruction};
use solana_msg::msg;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;
