//!
//!
//! To create, update, or close compressed accounts,
//! programs need to invoke the light system program via cross program invocation (cpi).
//!
//! ```ignore
//! declare_id!("2tzfijPBGbrR5PboyFUFKzfEoLTwdDSHUjANCw929wyt");
//! pub const LIGHT_CPI_SIGNER: CpiSigner =
//!   derive_light_cpi_signer!("2tzfijPBGbrR5PboyFUFKzfEoLTwdDSHUjANCw929wyt");
//!
//! let light_cpi_accounts = CpiAccounts::new(
//!     ctx.accounts.fee_payer.as_ref(),
//!     ctx.remaining_accounts,
//!     crate::LIGHT_CPI_SIGNER,
//! )
//! .map_err(ProgramError::from)?;
//!
//! let (address, address_seed) = derive_address(
//!     &[b"compressed", name.as_bytes()],
//!     &address_tree_info.get_tree_pubkey(&light_cpi_accounts)?,
//!     &crate::ID,
//! );
//! let new_address_params = address_tree_info.into_new_address_params_packed(address_seed);
//!
//! let mut my_compressed_account = LightAccount::<'_, MyCompressedAccount>::new_init(
//!     &crate::ID,
//!     Some(address),
//!     output_tree_index,
//! );
//!
//! my_compressed_account.name = name;
//! my_compressed_account.nested = NestedData::default();
//!
//! let cpi_inputs = CpiInputs::new_with_address(
//!     proof,
//!     // add compressed accounts to create, update or close here
//!     vec![my_compressed_account
//!         .to_account_info()
//!         .map_err(ProgramError::from)?],
//!     // add new addresses here
//!     // (existing addresses are part of the account info and must not be added here)
//!     vec![new_address_params],
//! );
//!
//! cpi_inputs
//!     .invoke_light_system_program(light_cpi_accounts)
//!     .map_err(ProgramError::from)?;
//! ```

mod accounts;
#[cfg(feature = "small_ix")]
mod accounts_small_ix;
mod invoke;

pub use accounts::*;
#[cfg(feature = "small_ix")]
pub use accounts_small_ix::*;
pub use invoke::*;
/// Derives cpi signer and bump to invoke the light system program at compile time.
pub use light_sdk_macros::derive_light_cpi_signer;
/// Contains program id, derived cpi signer, and bump,
pub use light_sdk_types::CpiSigner;
