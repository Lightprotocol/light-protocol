//! The base library to use Compressed Accounts in Solana on-chain Rust and Anchor programs.
//!
//! Compressed Accounts stores state as account hashes in State Merkle trees.
//! and unique addresses in Address Merkle trees.
//! Validity proofs (zero-knowledge proofs) verify that compressed account
//! state exists and new addresses do not exist yet.
//!
//! - No rent exemption payment required.
//! - Constant 128-byte validity proof per transaction for one or multiple compressed accounts and addresses.
//! - Compressed account data is sent as instruction data when accessed.
//! - State and address trees are managed by the protocol.
//!
//! For full program examples, see the [Program Examples](https://github.com/Lightprotocol/program-examples).
//! For detailed documentation, visit [zkcompression.com](https://www.zkcompression.com/).
//! For pinocchio solana program development see [`light-sdk-pinocchio`](https://docs.rs/light-sdk-pinocchio).
//! For rust client developement see [`light-client`](https://docs.rs/light-client).
//! For rust program testing see [`light-program-test`](https://docs.rs/light-program-test).
//! For local test validator with light system programs see [Light CLI](https://www.npmjs.com/package/@lightprotocol/zk-compression-cli).
//!
//! #  Using Compressed Accounts in Solana Programs
//! 1. [`Instruction`](crate::instruction)
//!     - [`CompressedAccountMeta`](crate::instruction::account_meta::CompressedAccountMeta) - Compressed account metadata structs for instruction data.
//!     - [`PackedAccounts`](crate::instruction::PackedAccounts) - Abstraction to prepare accounts offchain for instructions with compressed accounts.
//!     - [`ValidityProof`](crate::instruction::ValidityProof) - Proves that new addresses don't exist yet, and compressed account state exists.
//! 2. Compressed Account in Program
//!     - [`LightAccount`](crate::account) - Compressed account abstraction similar to anchor Account.
//!     - [`derive_address`](crate::address) - Create a compressed account address.
//!     - [`LightDiscriminator`] - DeriveMacro to derive a compressed account discriminator.
//! 3. [`Cpi`](crate::cpi)
//!     - [`CpiAccounts`](crate::cpi::v1::CpiAccounts) - Prepare accounts to cpi the light system program.
//!     - [`LightSystemProgramCpi`](crate::cpi::v1::LightSystemProgramCpi) - Prepare instruction data to cpi the light system program.
//!     - [`InvokeLightSystemProgram::invoke`](crate::cpi) - Invoke the light system program via cpi.
//!
//! ```text
//!  ├─ 𝐂𝐥𝐢𝐞𝐧𝐭
//!  │  ├─ Get ValidityProof from RPC.
//!  │  ├─ pack accounts with PackedAccounts into PackedAddressTreeInfo and PackedStateTreeInfo.
//!  │  ├─ pack CompressedAccountMeta.
//!  │  ├─ Build Instruction from PackedAccounts and CompressedAccountMetas.
//!  │  └─ Send transaction.
//!  │
//!  └─ 𝐂𝐮𝐬𝐭𝐨𝐦 𝐏𝐫𝐨𝐠𝐫𝐚𝐦
//!     ├─ CpiAccounts parse accounts consistent with PackedAccounts.
//!     ├─ LightAccount instantiates from CompressedAccountMeta.
//!     │
//!     └─ 𝐋𝐢𝐠𝐡𝐭 𝐒𝐲𝐬𝐭𝐞𝐦 𝐏𝐫𝐨𝐠𝐫𝐚𝐦 𝐂𝐏𝐈
//!        ├─ Verify ValidityProof.
//!        ├─ Update State Merkle tree.
//!        ├─ Update Address Merkle tree.
//!        └─ Complete atomic state transition.
//! ```
//!
//! # Features
//! 1. `anchor` - Derives AnchorSerialize, AnchorDeserialize instead of BorshSerialize, BorshDeserialize.
//!
//! 2. `v2`
//!     - available on devnet, localnet, and light-program-test.
//!     - Support for optimized v2 light system program instructions.
//!
//! 3. `cpi-context` - Enables CPI context operations for batched compressed account operations.
//!    - available on devnet, localnet, and light-program-test.
//!    - Enables the use of one validity proof across multiple cpis from different programs in one instruction.
//!    - For example spending compressed tokens (owned by the ctoken program) and updating a compressed pda (owned by a custom program)
//!      with one validity proof.
//!    - An instruction should not use more than one validity proof.
//!    - Requires the v2 feature.
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

/// Compressed account abstraction similar to anchor Account.
pub mod account;
pub use account::sha::LightAccount;

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

#[cfg(feature = "merkle-tree")]
pub mod merkle_tree;

#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
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
