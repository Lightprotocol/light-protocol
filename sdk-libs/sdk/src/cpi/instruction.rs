use light_compressed_account::instruction_data::compressed_proof::ValidityProof;

use crate::{
    account::LightAccount, AnchorDeserialize, AnchorSerialize, DataHasher, LightDiscriminator,
    ProgramError,
};

/// Trait for Light CPI instruction types
pub trait LightCpiInstruction: Sized {
    fn new_cpi(cpi_signer: crate::cpi::CpiSigner, proof: ValidityProof) -> Self;

    #[must_use = "with_light_account returns a new value"]
    fn with_light_account<A>(self, account: LightAccount<'_, A>) -> Result<Self, ProgramError>
    where
        A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + Default;

    #[must_use = "with_light_account_poseidon returns a new value"]
    fn with_light_account_poseidon<A>(
        self,
        account: crate::account::poseidon::LightAccount<'_, A>,
    ) -> Result<Self, ProgramError>
    where
        A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + DataHasher + Default;

    fn get_mode(&self) -> u8;
    fn get_bump(&self) -> u8;

    /// Writes instruction to CPI context as the first operation in a batch.
    ///
    /// # Availability
    /// Only available with the `cpi-context` feature enabled.
    #[cfg(feature = "cpi-context")]
    #[must_use = "write_to_cpi_context_first returns a new value"]
    fn write_to_cpi_context_first(self) -> Self;

    /// Writes instruction to CPI context as a subsequent operation in a batch.
    ///
    /// # Availability
    /// Only available with the `cpi-context` feature enabled.
    #[cfg(feature = "cpi-context")]
    #[must_use = "write_to_cpi_context_set returns a new value"]
    fn write_to_cpi_context_set(self) -> Self;

    /// Executes all operations accumulated in CPI context.
    ///
    /// # Availability
    /// Only available with the `cpi-context` feature enabled.
    #[cfg(feature = "cpi-context")]
    #[must_use = "execute_with_cpi_context returns a new value"]
    fn execute_with_cpi_context(self) -> Self;

    /// Returns whether this instruction uses CPI context.
    ///
    /// # Availability
    /// Only available with the `cpi-context` feature enabled.
    #[cfg(feature = "cpi-context")]
    fn get_with_cpi_context(&self) -> bool;

    /// Returns the CPI context configuration.
    ///
    /// # Availability
    /// Only available with the `cpi-context` feature enabled.
    #[cfg(feature = "cpi-context")]
    fn get_cpi_context(
        &self,
    ) -> &light_compressed_account::instruction_data::cpi_context::CompressedCpiContext;
}
