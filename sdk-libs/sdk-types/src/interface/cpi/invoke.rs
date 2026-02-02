//! Generic Light system program invocation.

use crate::constants::{CPI_AUTHORITY_PDA_SEED, LIGHT_SYSTEM_PROGRAM_ID};
use light_account_checks::{AccountInfoTrait, CpiMeta};
pub use light_compressed_account::LightInstructionData;

use crate::error::LightSdkTypesError;
use crate::interface::cpi::{account::CpiAccountsTrait, instruction::LightCpi};

/// Trait for invoking the Light system program via CPI.
///
/// Provides `invoke`, `invoke_write_to_cpi_context_first`,
/// `invoke_write_to_cpi_context_set`, and `invoke_execute_cpi_context` methods.
///
/// Blanket-implemented for all types implementing `LightInstructionData + LightCpi`.
pub trait InvokeLightSystemProgram {
    fn invoke<AI: AccountInfoTrait + Clone>(
        self,
        accounts: impl CpiAccountsTrait<AI>,
    ) -> Result<(), LightSdkTypesError>;
    fn invoke_write_to_cpi_context_first<AI: AccountInfoTrait + Clone>(
        self,
        accounts: impl CpiAccountsTrait<AI>,
    ) -> Result<(), LightSdkTypesError>;
    fn invoke_write_to_cpi_context_set<AI: AccountInfoTrait + Clone>(
        self,
        accounts: impl CpiAccountsTrait<AI>,
    ) -> Result<(), LightSdkTypesError>;
    fn invoke_execute_cpi_context<AI: AccountInfoTrait + Clone>(
        self,
        accounts: impl CpiAccountsTrait<AI>,
    ) -> Result<(), LightSdkTypesError>;
}

impl<T> InvokeLightSystemProgram for T
where
    T: LightInstructionData + LightCpi,
{
    fn invoke<AI: AccountInfoTrait + Clone>(
        self,
        accounts: impl CpiAccountsTrait<AI>,
    ) -> Result<(), LightSdkTypesError> {
        // Check if CPI context operations are being attempted
        {
            use light_compressed_account::instruction_data::cpi_context::CompressedCpiContext;
            if self.get_with_cpi_context()
                || *self.get_cpi_context() == CompressedCpiContext::set()
                || *self.get_cpi_context() == CompressedCpiContext::first()
            {
                return Err(LightSdkTypesError::InvalidInstructionData);
            }
        }

        // Validate mode consistency
        if let Some(account_mode) = accounts.get_mode() {
            if account_mode != self.get_mode() {
                return Err(LightSdkTypesError::InvalidInstructionData);
            }
        }

        let data = self.data().map_err(LightSdkTypesError::from)?;
        let account_infos = accounts.to_account_infos();
        let account_metas = accounts.to_account_metas()?;

        invoke_light_system_program::<AI>(&account_infos, &account_metas, &data, self.get_bump())
    }

    fn invoke_write_to_cpi_context_first<AI: AccountInfoTrait + Clone>(
        self,
        accounts: impl CpiAccountsTrait<AI>,
    ) -> Result<(), LightSdkTypesError> {
        let instruction_data = self.write_to_cpi_context_first();
        inner_invoke_write_to_cpi_context_typed(instruction_data, accounts)
    }

    fn invoke_write_to_cpi_context_set<AI: AccountInfoTrait + Clone>(
        self,
        accounts: impl CpiAccountsTrait<AI>,
    ) -> Result<(), LightSdkTypesError> {
        let instruction_data = self.write_to_cpi_context_set();
        inner_invoke_write_to_cpi_context_typed(instruction_data, accounts)
    }

    fn invoke_execute_cpi_context<AI: AccountInfoTrait + Clone>(
        self,
        accounts: impl CpiAccountsTrait<AI>,
    ) -> Result<(), LightSdkTypesError> {
        let instruction_data = self.execute_with_cpi_context();

        let data = instruction_data.data().map_err(LightSdkTypesError::from)?;
        let account_infos = accounts.to_account_infos();
        let account_metas = accounts.to_account_metas()?;

        invoke_light_system_program::<AI>(
            &account_infos,
            &account_metas,
            &data,
            instruction_data.get_bump(),
        )
    }
}

/// Inner helper for write_to_cpi_context operations.
fn inner_invoke_write_to_cpi_context_typed<AI, T>(
    instruction_data: T,
    accounts: impl CpiAccountsTrait<AI>,
) -> Result<(), LightSdkTypesError>
where
    AI: AccountInfoTrait + Clone,
    T: LightInstructionData + LightCpi,
{
    if instruction_data.has_read_only_accounts() {
        return Err(LightSdkTypesError::ReadOnlyAccountsNotSupportedInCpiContext);
    }

    let data = instruction_data.data().map_err(LightSdkTypesError::from)?;
    let account_infos = accounts.to_account_infos();

    if account_infos.len() < 3 {
        return Err(LightSdkTypesError::NotEnoughAccountKeys);
    }

    let account_metas = vec![
        CpiMeta {
            pubkey: account_infos[0].key(),
            is_signer: true,
            is_writable: true,
        },
        CpiMeta {
            pubkey: account_infos[1].key(),
            is_signer: true,
            is_writable: false,
        },
        CpiMeta {
            pubkey: account_infos[2].key(),
            is_signer: false,
            is_writable: true,
        },
    ];

    invoke_light_system_program::<AI>(
        &account_infos,
        &account_metas,
        &data,
        instruction_data.get_bump(),
    )
}

/// Low-level function to invoke the Light system program with a PDA signer.
///
/// Uses `AI::invoke_cpi()` to be generic over the runtime backend.
#[inline(always)]
pub fn invoke_light_system_program<AI: AccountInfoTrait + Clone>(
    account_infos: &[AI],
    account_metas: &[CpiMeta],
    data: &[u8],
    bump: u8,
) -> Result<(), LightSdkTypesError> {
    let signer_seeds: &[&[u8]] = &[CPI_AUTHORITY_PDA_SEED, &[bump]];
    AI::invoke_cpi(
        &LIGHT_SYSTEM_PROGRAM_ID,
        data,
        account_metas,
        account_infos,
        &[signer_seeds],
    )
    .map_err(|_| LightSdkTypesError::CpiFailed)
}
