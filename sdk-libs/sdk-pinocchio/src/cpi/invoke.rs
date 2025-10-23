pub use light_compressed_account::LightInstructionData;
use light_sdk_types::constants::{CPI_AUTHORITY_PDA_SEED, LIGHT_SYSTEM_PROGRAM_ID};
#[cfg(any(feature = "std", feature = "alloc"))]
use pinocchio::pubkey::Pubkey;
use pinocchio::{
    cpi::slice_invoke_signed,
    instruction::{Instruction, Seed, Signer},
    program_error::ProgramError,
};

use super::{account::CpiAccountsTrait, instruction::LightCpiInstruction};
#[cfg(any(feature = "std", feature = "alloc"))]
use crate::error::LightSdkError;

pub trait InvokeLightSystemProgram {
    #[cfg(any(feature = "std", feature = "alloc"))]
    fn invoke(self, accounts: impl CpiAccountsTrait) -> Result<(), ProgramError>;

    fn invoke_array<const N: usize>(
        self,
        accounts: impl CpiAccountsTrait,
    ) -> Result<(), ProgramError>;
}

// Blanket implementation for types that implement both LightInstructionData and LightCpiInstruction
impl<T> InvokeLightSystemProgram for T
where
    T: LightInstructionData + LightCpiInstruction,
{
    #[cfg(any(feature = "std", feature = "alloc"))]
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

    fn invoke_array<const N: usize>(
        self,
        accounts: impl CpiAccountsTrait,
    ) -> Result<(), ProgramError> {
        use light_compressed_account::CompressedAccountError;
        // Validate mode consistency
        if accounts.get_mode() != self.get_mode() {
            return Err(ProgramError::InvalidArgument);
        }

        // Serialize instruction data with discriminator using data_array
        let data = self.data_array::<N>().map_err(|e| match e {
            CompressedAccountError::InputTooLarge(_) => ProgramError::InvalidInstructionData,
            _ => ProgramError::InvalidArgument,
        })?;

        // Get account infos and metas
        let account_infos = accounts
            .to_account_infos_for_invoke()
            .map_err(ProgramError::from)?;
        let account_metas = accounts.to_account_metas().map_err(ProgramError::from)?;

        let instruction = Instruction {
            program_id: &LIGHT_SYSTEM_PROGRAM_ID,
            accounts: &account_metas,
            data: data.as_slice(),
        };

        invoke_light_system_program(&account_infos, instruction, self.get_bump())
    }
}

/// Low-level function to invoke the Light system program with a PDA signer.
///
/// **Note**: This is a low-level function. In most cases, you should use the
/// [`InvokeLightSystemProgram`] trait methods instead, which provide a higher-level
/// interface with better type safety and ergonomics.
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
