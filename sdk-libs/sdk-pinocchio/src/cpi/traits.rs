#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;
#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
pub use light_compressed_account::LightInstructionData;
use light_sdk_types::constants::{CPI_AUTHORITY_PDA_SEED, LIGHT_SYSTEM_PROGRAM_ID};
use pinocchio::{
    cpi::slice_invoke_signed,
    instruction::{AccountMeta, Instruction, Seed, Signer},
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::error::LightSdkError;

/// Trait for types that can provide account information for CPI calls
pub trait CpiAccountsTrait {
    /// Convert to a vector of AccountMeta references for instruction
    fn to_account_metas(&self) -> crate::error::Result<Vec<AccountMeta<'_>>>;

    /// Convert to account infos for invoke
    fn to_account_infos_for_invoke(
        &self,
    ) -> crate::error::Result<Vec<&pinocchio::account_info::AccountInfo>>;

    /// Get the CPI signer bump
    fn bump(&self) -> u8;

    /// Get the mode for the instruction (0 for v1, 1 for v2)
    fn get_mode(&self) -> u8;
}

/// Trait for Light CPI instruction types
pub trait LightCpiInstruction: Sized {
    fn new_cpi(cpi_signer: light_sdk_types::CpiSigner, proof: ValidityProof) -> Self;

    #[cfg(feature = "light-account")]
    fn with_light_account<A>(
        self,
        account: crate::LightAccount<'_, A>,
    ) -> Result<Self, ProgramError>
    where
        A: borsh::BorshSerialize
            + borsh::BorshDeserialize
            + crate::LightDiscriminator
            + light_hasher::DataHasher
            + Default;

    fn get_mode(&self) -> u8;
    fn get_bump(&self) -> u8;
}

pub trait InvokeLightSystemProgram {
    fn invoke(self, accounts: impl CpiAccountsTrait) -> Result<(), ProgramError>;
}

// Blanket implementation for types that implement both LightInstructionData and LightCpiInstruction
impl<T> InvokeLightSystemProgram for T
where
    T: LightInstructionData + LightCpiInstruction,
{
    fn invoke(self, accounts: impl CpiAccountsTrait) -> Result<(), ProgramError> {
        // Validate mode consistency
        if accounts.get_mode() != self.get_mode() {
            return Err(ProgramError::from(LightSdkError::ModeMismatch));
        }

        // Serialize instruction data with discriminator
        let data = self
            .data()
            .map_err(|e| ProgramError::from(LightSdkError::from(e)))?;

        // Get account infos and metas
        let account_infos = accounts
            .to_account_infos_for_invoke()
            .map_err(ProgramError::from)?;
        let account_metas = accounts.to_account_metas().map_err(ProgramError::from)?;

        let instruction = Instruction {
            program_id: &Pubkey::from(LIGHT_SYSTEM_PROGRAM_ID),
            accounts: &account_metas,
            data: &data,
        };

        invoke_light_system_program(&account_infos, instruction, self.get_bump())
    }
}

#[inline(always)]
pub fn invoke_light_system_program(
    account_infos: &[&pinocchio::account_info::AccountInfo],
    instruction: Instruction,
    bump: u8,
) -> Result<(), ProgramError> {
    let bump_seed = [bump];
    let seed_array = [
        Seed::from(CPI_AUTHORITY_PDA_SEED),
        Seed::from(bump_seed.as_slice()),
    ];
    let signer = Signer::from(&seed_array);

    slice_invoke_signed(&instruction, account_infos, &[signer])
}
