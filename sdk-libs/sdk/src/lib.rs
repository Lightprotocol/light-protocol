//! SDK for building programs with compressed accounts on Solana.
//!
//! State is stored as hashes in Merkle trees. Validity proofs verify state exists.
//! No rent required. Constant 128-byte proof per transaction.
//!
//! See [zkcompression.com](https://www.zkcompression.com/) for docs and [Program Examples](https://github.com/Lightprotocol/program-examples).
//!
//! Related crates:
//! - [`light-sdk-pinocchio`](https://docs.rs/light-sdk-pinocchio) - Pinocchio programs
//! - [`light-client`](https://docs.rs/light-client) - Client development
//! - [`light-program-test`](https://docs.rs/light-program-test) - Testing
//!
//! # Main modules
//! - [`instruction`] - Build instructions with compressed accounts
//! - [`account`] - LightAccount abstraction
//! - [`address`] - Derive compressed addresses
//! - [`cpi`] - CPI to light system program
//!
//! # Features
//! - `anchor` - Use AnchorSerialize/AnchorDeserialize
//! - `v2` - Optimized v2 instructions (devnet, localnet)
//! - `cpi-context` - Share one validity proof across multiple CPIs (requires v2)
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
//!         let mut my_compressed_account = LightAccount::<CounterAccount>::new_init(
//!             &crate::ID,
//!             Some(address),
//!             output_tree_index,
//!         );
//!
//!         my_compressed_account.owner = ctx.accounts.fee_payer.key();
//!
//!         LightSystemProgramCpi::new_cpi(crate::LIGHT_CPI_SIGNER, proof)
//!             .with_light_account(my_compressed_account)?
//!             .with_new_addresses(&[new_address_params])
//!             .invoke(light_cpi_accounts)
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

pub mod account;
pub use account::sha::LightAccount;

pub mod address;
pub mod cpi;
pub mod error;
pub mod instruction;
pub mod legacy;
pub mod proof;
pub mod transfer;
pub mod utils;

pub use proof::borsh_compat;
pub mod compressible;
#[cfg(feature = "merkle-tree")]
pub mod merkle_tree;

#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use compressible::{
    process_initialize_compression_config_account_info,
    process_initialize_compression_config_checked, process_update_compression_config, CompressAs,
    CompressedInitSpace, CompressibleConfig, CompressionInfo, HasCompressionInfo, Pack, Space,
    Unpack, COMPRESSIBLE_CONFIG_SEED, MAX_ADDRESS_TREES_PER_SPACE,
};
pub use light_account_checks::{self, discriminator::Discriminator as LightDiscriminator};
pub use light_hasher;
#[cfg(feature = "poseidon")]
use light_hasher::DataHasher;
pub use light_macros::{derive_light_cpi_signer, derive_light_cpi_signer_pda};
pub use light_sdk_macros::{
    light_system_accounts, LightDiscriminator, LightHasher, LightHasherSha, LightTraits,
};
pub use light_sdk_types::{constants, CpiSigner};
use solana_account_info::AccountInfo;
use solana_cpi::invoke_signed;
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

pub trait PubkeyTrait {
    fn to_solana_pubkey(&self) -> Pubkey;
    fn to_array(&self) -> [u8; 32];
}

impl PubkeyTrait for [u8; 32] {
    fn to_solana_pubkey(&self) -> Pubkey {
        Pubkey::from(*self)
    }

    fn to_array(&self) -> [u8; 32] {
        *self
    }
}

#[cfg(not(feature = "anchor"))]
impl PubkeyTrait for Pubkey {
    fn to_solana_pubkey(&self) -> Pubkey {
        *self
    }

    fn to_array(&self) -> [u8; 32] {
        self.to_bytes()
    }
}

#[cfg(feature = "anchor")]
impl PubkeyTrait for anchor_lang::prelude::Pubkey {
    fn to_solana_pubkey(&self) -> Pubkey {
        Pubkey::from(self.to_bytes())
    }

    fn to_array(&self) -> [u8; 32] {
        self.to_bytes()
    }
}
