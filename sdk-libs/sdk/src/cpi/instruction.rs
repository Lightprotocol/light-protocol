use light_compressed_account::instruction_data::compressed_proof::ValidityProof;

#[cfg(feature = "poseidon")]
use crate::DataHasher;
use crate::{
    account::LightAccount, AnchorDeserialize, AnchorSerialize, LightDiscriminator, ProgramError,
};

/// Trait for Light CPI instruction types
pub trait LightCpiInstruction: Sized {
    /// Creates a new CPI instruction builder with a validity proof.
    ///
    /// # Arguments
    /// * `cpi_signer` - The CPI signer containing program ID and bump seed
    /// * `proof` - Validity proof for compressed account operations
    fn new_cpi(cpi_signer: crate::cpi::CpiSigner, proof: ValidityProof) -> Self;

    /// Adds a compressed account to the instruction (using SHA256 hashing).
    ///
    /// The account can be an input (for updating/closing), output (for creating/updating),
    /// or both. The method automatically handles the conversion based on the account state.
    ///
    /// # Arguments
    /// * `account` - The light account to add to the instruction
    ///
    /// # Type Parameters
    /// * `A` - The compressed account data type
    #[must_use = "with_light_account returns a new value"]
    fn with_light_account<A>(self, account: LightAccount<A>) -> Result<Self, ProgramError>
    where
        A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + Default;

    /// Adds a compressed account to the instruction (using Poseidon hashing).
    ///
    /// Similar to [`with_light_account`](Self::with_light_account), but uses Poseidon hashing
    /// instead of SHA256. Use this when your compressed account data implements [`DataHasher`].
    ///
    /// # Arguments
    /// * `account` - The light account to add to the instruction
    ///
    /// # Type Parameters
    /// * `A` - The compressed account data type that implements DataHasher
    #[cfg(feature = "poseidon")]
    #[must_use = "with_light_account_poseidon returns a new value"]
    fn with_light_account_poseidon<A>(
        self,
        account: crate::account::poseidon::LightAccount<A>,
    ) -> Result<Self, ProgramError>
    where
        A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + DataHasher + Default;

    /// Returns the instruction mode (0 for v1, 1 for v2).
    fn get_mode(&self) -> u8;

    /// Returns the CPI signer bump seed.
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

    /// Returns whether this instruction has any read-only accounts.
    ///
    /// # Availability
    /// Only available with the `cpi-context` feature enabled.
    #[cfg(feature = "cpi-context")]
    fn has_read_only_accounts(&self) -> bool;
}
