pub use light_compressed_account::LightInstructionData;
use light_sdk_types::constants::{CPI_AUTHORITY_PDA_SEED, LIGHT_SYSTEM_PROGRAM_ID};

#[cfg(feature = "cpi-context")]
use crate::AccountMeta;
use crate::{
    cpi::{account::CpiAccountsTrait, instruction::LightCpiInstruction},
    error::LightSdkError,
    invoke_signed, AccountInfo, Instruction, ProgramError,
};

pub trait InvokeLightSystemProgram {
    fn invoke<'info>(self, accounts: impl CpiAccountsTrait<'info>) -> Result<(), ProgramError>;

    #[cfg(feature = "cpi-context")]
    fn invoke_write_to_cpi_context_first<'info>(
        self,
        accounts: impl CpiAccountsTrait<'info>,
    ) -> Result<(), ProgramError>;

    #[cfg(feature = "cpi-context")]
    fn invoke_write_to_cpi_context_set<'info>(
        self,
        accounts: impl CpiAccountsTrait<'info>,
    ) -> Result<(), ProgramError>;

    #[cfg(feature = "cpi-context")]
    fn invoke_execute_cpi_context<'info>(
        self,
        accounts: impl CpiAccountsTrait<'info>,
    ) -> Result<(), ProgramError>;
}

// Blanket implementation for types that implement both LightInstructionData and LightCpiInstruction
impl<T> InvokeLightSystemProgram for T
where
    T: LightInstructionData + LightCpiInstruction,
{
    fn invoke<'info>(self, accounts: impl CpiAccountsTrait<'info>) -> Result<(), ProgramError> {
        #[cfg(feature = "cpi-context")]
        {
            // Check if CPI context operations are being attempted
            use light_compressed_account::instruction_data::cpi_context::CompressedCpiContext;
            if self.get_with_cpi_context()
                || *self.get_cpi_context() == CompressedCpiContext::set()
                || *self.get_cpi_context() == CompressedCpiContext::first()
            {
                solana_msg::msg!(
                    "CPI context operations not supported in invoke(). Use invoke_write_to_cpi_context_first(), invoke_write_to_cpi_context_set(), or invoke_execute_cpi_context() instead"
                );
                return Err(ProgramError::InvalidInstructionData);
            }
        }

        // Validate mode consistency
        if let Some(account_mode) = accounts.get_mode() {
            if account_mode != self.get_mode() {
                solana_msg::msg!(
                    "Mode mismatch: accounts have mode {} but instruction data has mode {}",
                    account_mode,
                    self.get_mode()
                );
                return Err(ProgramError::InvalidInstructionData);
            }
        }

        // Serialize instruction data with discriminator
        let data = self
            .data()
            .map_err(LightSdkError::from)
            .map_err(ProgramError::from)?;

        // Get account infos and metas
        let account_infos = accounts.to_account_infos();
        let account_metas = accounts.to_account_metas()?;

        let instruction = Instruction {
            program_id: LIGHT_SYSTEM_PROGRAM_ID.into(),
            accounts: account_metas,
            data,
        };

        invoke_light_system_program(&account_infos, instruction, self.get_bump())
    }

    #[cfg(feature = "cpi-context")]
    fn invoke_write_to_cpi_context_first<'info>(
        self,
        accounts: impl CpiAccountsTrait<'info>,
    ) -> Result<(), ProgramError> {
        let instruction_data = self.write_to_cpi_context_first();
        inner_invoke_write_to_cpi_context_typed(instruction_data, accounts)
    }

    #[cfg(feature = "cpi-context")]
    fn invoke_write_to_cpi_context_set<'info>(
        self,
        accounts: impl CpiAccountsTrait<'info>,
    ) -> Result<(), ProgramError> {
        let instruction_data = self.write_to_cpi_context_set();
        inner_invoke_write_to_cpi_context_typed(instruction_data, accounts)
    }

    #[cfg(feature = "cpi-context")]
    fn invoke_execute_cpi_context<'info>(
        self,
        accounts: impl CpiAccountsTrait<'info>,
    ) -> Result<(), ProgramError> {
        let instruction_data = self.execute_with_cpi_context();
        // Serialize instruction data with discriminator
        let data = instruction_data
            .data()
            .map_err(LightSdkError::from)
            .map_err(ProgramError::from)?;

        // Get account infos and metas
        let account_infos = accounts.to_account_infos();
        let account_metas = accounts.to_account_metas()?;

        let instruction = Instruction {
            program_id: LIGHT_SYSTEM_PROGRAM_ID.into(),
            accounts: account_metas,
            data,
        };
        invoke_light_system_program(&account_infos, instruction, instruction_data.get_bump())
    }
}

// Generic inner helper for write_to_cpi_context operations
#[cfg(feature = "cpi-context")]
#[inline(always)]
fn inner_invoke_write_to_cpi_context_typed<'info, T>(
    instruction_data: T,
    accounts: impl CpiAccountsTrait<'info>,
) -> Result<(), ProgramError>
where
    T: LightInstructionData + LightCpiInstruction,
{
    // Check if read-only accounts are present
    if instruction_data.has_read_only_accounts() {
        solana_msg::msg!(
            "Read-only accounts are not supported in write_to_cpi_context operations. Use invoke_execute_cpi_context() instead."
        );
        return Err(LightSdkError::ReadOnlyAccountsNotSupportedInCpiContext.into());
    }

    // Serialize instruction data with discriminator
    let data = instruction_data
        .data()
        .map_err(LightSdkError::from)
        .map_err(ProgramError::from)?;

    // Get account infos and metas
    let account_infos = accounts.to_account_infos();

    // Extract account pubkeys from account_infos
    // Assuming order: [fee_payer, authority, cpi_context, ...]
    if account_infos.len() < 3 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let instruction = Instruction {
        program_id: LIGHT_SYSTEM_PROGRAM_ID.into(),
        accounts: vec![
            AccountMeta {
                pubkey: *account_infos[0].key, // fee_payer
                is_writable: true,
                is_signer: true,
            },
            AccountMeta {
                pubkey: *account_infos[1].key, // authority
                is_writable: false,
                is_signer: true,
            },
            AccountMeta {
                pubkey: *account_infos[2].key, // cpi_context
                is_writable: true,
                is_signer: false,
            },
        ],
        data,
    };

    invoke_light_system_program(&account_infos, instruction, instruction_data.get_bump())
}

/// Low-level function to invoke the Light system program with a PDA signer.
///
/// **Note**: This is a low-level function. In most cases, you should use the
/// [`InvokeLightSystemProgram`] trait methods instead, which provide a higher-level
/// interface with better type safety and ergonomics.
#[inline(always)]
pub fn invoke_light_system_program(
    account_infos: &[AccountInfo],
    instruction: Instruction,
    bump: u8,
) -> Result<(), ProgramError> {
    let signer_seeds = [CPI_AUTHORITY_PDA_SEED, &[bump]];
    invoke_signed(&instruction, account_infos, &[signer_seeds.as_slice()])
}
