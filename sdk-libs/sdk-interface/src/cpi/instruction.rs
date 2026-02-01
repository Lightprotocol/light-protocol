use light_compressed_account::instruction_data::compressed_proof::ValidityProof;

/// Base trait for Light CPI instruction types.
///
/// This is the framework-agnostic version that provides CPI builder methods
/// without referencing SDK-specific types like `LightAccount`.
///
/// Each SDK (`light-sdk`, `light-sdk-pinocchio`) defines its own
/// `LightCpiInstruction` trait that includes `with_light_account`.
pub trait LightCpi: Sized {
    /// Creates a new CPI instruction builder with a validity proof.
    ///
    /// # Arguments
    /// * `cpi_signer` - The CPI signer containing program ID and bump seed
    /// * `proof` - Validity proof for compressed account operations
    fn new_cpi(cpi_signer: crate::cpi::CpiSigner, proof: ValidityProof) -> Self;

    /// Returns the instruction mode (0 for v1, 1 for v2).
    fn get_mode(&self) -> u8;

    /// Returns the CPI signer bump seed.
    fn get_bump(&self) -> u8;

    /// Writes instruction to CPI context as the first operation in a batch.
    ///
    /// # Availability
    /// Only available with the `cpi-context` feature enabled.
    #[must_use = "write_to_cpi_context_first returns a new value"]
    fn write_to_cpi_context_first(self) -> Self;

    /// Writes instruction to CPI context as a subsequent operation in a batch.
    ///
    /// # Availability
    /// Only available with the `cpi-context` feature enabled.
    #[must_use = "write_to_cpi_context_set returns a new value"]
    fn write_to_cpi_context_set(self) -> Self;

    /// Executes all operations accumulated in CPI context.
    ///
    /// # Availability
    /// Only available with the `cpi-context` feature enabled.
    #[must_use = "execute_with_cpi_context returns a new value"]
    fn execute_with_cpi_context(self) -> Self;

    /// Returns whether this instruction uses CPI context.
    ///
    /// # Availability
    /// Only available with the `cpi-context` feature enabled.
    fn get_with_cpi_context(&self) -> bool;

    /// Returns the CPI context configuration.
    ///
    /// # Availability
    /// Only available with the `cpi-context` feature enabled.
    fn get_cpi_context(
        &self,
    ) -> &light_compressed_account::instruction_data::cpi_context::CompressedCpiContext;

    /// Returns whether this instruction has any read-only accounts.
    ///
    /// # Availability
    /// Only available with the `cpi-context` feature enabled.
    fn has_read_only_accounts(&self) -> bool;
}
