pub use light_compressed_account::LightInstructionData;
use light_sdk_types::constants::{CPI_AUTHORITY_PDA_SEED, LIGHT_SYSTEM_PROGRAM_ID};
#[cfg(any(feature = "std", feature = "alloc"))]
use pinocchio::address::Address as Pubkey;
use pinocchio::{
    cpi::invoke_signed_with_slice,
    error::ProgramError,
    instruction::{
        cpi::{Seed, Signer},
        InstructionView,
    },
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

impl<T> InvokeLightSystemProgram for T
where
    T: LightInstructionData + LightCpiInstruction,
{
    #[cfg(any(feature = "std", feature = "alloc"))]
    fn invoke(self, accounts: impl CpiAccountsTrait) -> Result<(), ProgramError> {
        if accounts.get_mode() != self.get_mode() {
            return Err(ProgramError::from(LightSdkError::ModeMismatch));
        }

        let data = self
            .data()
            .map_err(|e| ProgramError::from(LightSdkError::from(e)))?;

        let account_infos = accounts
            .to_account_infos_for_invoke()
            .map_err(ProgramError::from)?;
        let account_metas = accounts.to_account_metas().map_err(ProgramError::from)?;
        let program_id = Pubkey::from(LIGHT_SYSTEM_PROGRAM_ID);
        let instruction = InstructionView {
            program_id: &program_id,
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
        if accounts.get_mode() != self.get_mode() {
            return Err(ProgramError::InvalidArgument);
        }

        let data = self.data_array::<N>().map_err(|e| match e {
            CompressedAccountError::InputTooLarge(_) => ProgramError::InvalidInstructionData,
            _ => ProgramError::InvalidArgument,
        })?;

        let account_infos = accounts
            .to_account_infos_for_invoke()
            .map_err(ProgramError::from)?;
        let account_metas = accounts.to_account_metas().map_err(ProgramError::from)?;
        let program_id = Pubkey::from(LIGHT_SYSTEM_PROGRAM_ID);
        let instruction = InstructionView {
            program_id: &program_id,
            accounts: &account_metas,
            data: data.as_slice(),
        };

        invoke_light_system_program(&account_infos, instruction, self.get_bump())
    }
}

#[inline(always)]
pub fn invoke_light_system_program(
    account_infos: &[&pinocchio::AccountView],
    instruction: InstructionView,
    bump: u8,
) -> Result<(), ProgramError> {
    let bump_seed = [bump];
    let seed_array = [
        Seed::from(CPI_AUTHORITY_PDA_SEED),
        Seed::from(bump_seed.as_slice()),
    ];
    let signer = Signer::from(&seed_array);

    invoke_signed_with_slice(&instruction, account_infos, &[signer])
}
