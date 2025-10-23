use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
pub use light_compressed_account::LightInstructionData;

/// Trait for Light CPI instruction types
pub trait LightCpiInstruction: Sized {
    fn new_cpi(cpi_signer: light_sdk_types::CpiSigner, proof: ValidityProof) -> Self;

    #[cfg(feature = "light-account")]
    fn with_light_account<A>(
        self,
        account: crate::LightAccount<A>,
    ) -> Result<Self, pinocchio::program_error::ProgramError>
    where
        A: borsh::BorshSerialize
            + borsh::BorshDeserialize
            + crate::LightDiscriminator
            + light_hasher::DataHasher
            + Default;

    fn get_mode(&self) -> u8;
    fn get_bump(&self) -> u8;
}
