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

// Re-export everything from interface's CPI module (LightCpi, InvokeLightSystemProgram, etc.)
pub use light_sdk_interface::cpi::*;

use light_compressed_account::instruction_data::{
    compressed_proof::ValidityProof,
    cpi_context::CompressedCpiContext,
};

use crate::{
    account::LightAccount, AnchorDeserialize, AnchorSerialize, LightDiscriminator, ProgramError,
};

#[cfg(feature = "poseidon")]
use crate::DataHasher;

/// Trait for Light CPI instruction types including `with_light_account`.
///
/// This is the SDK-level trait that provides the full builder API:
/// - CPI builder methods (`new_cpi`, `get_mode`, `get_bump`, etc.)
/// - `with_light_account` for adding compressed accounts
///
/// Internally delegates base methods to [`LightCpi`].
pub trait LightCpiInstruction: Sized {
    /// Creates a new CPI instruction builder with a validity proof.
    fn new_cpi(cpi_signer: crate::cpi::CpiSigner, proof: ValidityProof) -> Self;

    /// Returns the instruction mode (0 for v1, 1 for v2).
    fn get_mode(&self) -> u8;

    /// Returns the CPI signer bump seed.
    fn get_bump(&self) -> u8;

    /// Writes instruction to CPI context as the first operation in a batch.
    #[must_use = "write_to_cpi_context_first returns a new value"]
    fn write_to_cpi_context_first(self) -> Self;

    /// Writes instruction to CPI context as a subsequent operation in a batch.
    #[must_use = "write_to_cpi_context_set returns a new value"]
    fn write_to_cpi_context_set(self) -> Self;

    /// Executes all operations accumulated in CPI context.
    #[must_use = "execute_with_cpi_context returns a new value"]
    fn execute_with_cpi_context(self) -> Self;

    /// Returns whether this instruction uses CPI context.
    fn get_with_cpi_context(&self) -> bool;

    /// Returns the CPI context configuration.
    fn get_cpi_context(&self) -> &CompressedCpiContext;

    /// Returns whether this instruction has any read-only accounts.
    fn has_read_only_accounts(&self) -> bool;

    /// Adds a compressed account to the instruction (SHA256 hashing).
    #[must_use = "with_light_account returns a new value"]
    fn with_light_account<A>(self, account: LightAccount<A>) -> Result<Self, ProgramError>
    where
        A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + Default;

    /// Adds a compressed account to the instruction (Poseidon hashing).
    #[cfg(feature = "poseidon")]
    #[must_use = "with_light_account_poseidon returns a new value"]
    fn with_light_account_poseidon<A>(
        self,
        account: crate::account::poseidon::LightAccount<A>,
    ) -> Result<Self, ProgramError>
    where
        A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + DataHasher + Default;
}

/// Macro to delegate base LightCpi methods to the interface trait impl.
macro_rules! delegate_light_cpi {
    ($ty:ty) => {
        fn new_cpi(
            cpi_signer: crate::cpi::CpiSigner,
            proof: ValidityProof,
        ) -> Self {
            <$ty as light_sdk_interface::cpi::LightCpi>::new_cpi(cpi_signer, proof)
        }
        fn get_mode(&self) -> u8 {
            <$ty as light_sdk_interface::cpi::LightCpi>::get_mode(self)
        }
        fn get_bump(&self) -> u8 {
            <$ty as light_sdk_interface::cpi::LightCpi>::get_bump(self)
        }
        fn write_to_cpi_context_first(self) -> Self {
            <$ty as light_sdk_interface::cpi::LightCpi>::write_to_cpi_context_first(self)
        }
        fn write_to_cpi_context_set(self) -> Self {
            <$ty as light_sdk_interface::cpi::LightCpi>::write_to_cpi_context_set(self)
        }
        fn execute_with_cpi_context(self) -> Self {
            <$ty as light_sdk_interface::cpi::LightCpi>::execute_with_cpi_context(self)
        }
        fn get_with_cpi_context(&self) -> bool {
            <$ty as light_sdk_interface::cpi::LightCpi>::get_with_cpi_context(self)
        }
        fn get_cpi_context(&self) -> &CompressedCpiContext {
            <$ty as light_sdk_interface::cpi::LightCpi>::get_cpi_context(self)
        }
        fn has_read_only_accounts(&self) -> bool {
            <$ty as light_sdk_interface::cpi::LightCpi>::has_read_only_accounts(self)
        }
    };
}

pub(crate) use delegate_light_cpi;

pub mod v1;
#[cfg(feature = "v2")]
pub mod v2;
