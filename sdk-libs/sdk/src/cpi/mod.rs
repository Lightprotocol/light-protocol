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
//! )?;
//!
//! let (address, address_seed) = derive_address(
//!     &[b"compressed", name.as_bytes()],
//!     &address_tree_info.get_tree_pubkey(&light_cpi_accounts)?,
//!     &crate::ID,
//! );
//! let new_address_params = address_tree_info.into_new_address_params_packed(address_seed);
//!
//! let mut my_compressed_account = LightAccount::<MyCompressedAccount>::new_init(
//!     &crate::ID,
//!     Some(address),
//!     output_tree_index,
//! );
//!
//! my_compressed_account.name = name;
//!
//! LightSystemProgramCpi::new_cpi(crate::LIGHT_CPI_SIGNER, proof)
//!     .with_light_account(my_compressed_account)?
//!     .with_new_addresses(&[new_address_params])
//!     .invoke(light_cpi_accounts)?;
//! ```

// Re-export everything from interface's CPI module
pub use light_sdk_interface::cpi::*;

// SDK-specific extension trait (adds with_light_account to CPI builders)
mod instruction;
pub use instruction::WithLightAccount;

// V1/V2 modules that provide WithLightAccount impls
pub mod v1;
#[cfg(feature = "v2")]
pub mod v2;
