use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
pub use light_compressed_account::LightInstructionData;
use light_sdk_types::{
    constants::{CPI_AUTHORITY_PDA_SEED, LIGHT_SYSTEM_PROGRAM_ID},
    cpi_context_write::CpiContextWriteAccounts,
};

use crate::{
    account::LightAccount,
    cpi::{
        accounts_cpi_context::get_account_metas_from_config_cpi_context,
        get_account_metas_from_config, CpiAccounts, CpiInstructionConfig,
    },
    error::LightSdkError,
    invoke_signed, AccountInfo, AccountMeta, AnchorDeserialize, AnchorSerialize, DataHasher,
    Instruction, LightDiscriminator, ProgramError,
};

/// Trait for types that can provide account information for CPI calls
pub trait CpiAccountsTrait<'info> {
    /// Convert to a vector of AccountInfo references
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>>;

    /// Generate account metas
    fn to_account_metas(&self) -> Result<Vec<AccountMeta>, ProgramError>;

    /// Get the mode for the instruction (0 for regular, 1 for small, None if unknown)
    fn get_mode(&self) -> Option<u8>;
}

// Implementation for CpiAccounts
impl<'info> CpiAccountsTrait<'info> for CpiAccounts<'_, 'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        self.to_account_infos()
    }

    fn to_account_metas(&self) -> Result<Vec<AccountMeta>, ProgramError> {
        let config = CpiInstructionConfig::try_from(self).map_err(ProgramError::from)?;
        Ok(get_account_metas_from_config(config))
    }

    fn get_mode(&self) -> Option<u8> {
        Some(0) // Regular mode
    }
}

// Implementation for &[AccountInfo]
impl<'info> CpiAccountsTrait<'info> for &[AccountInfo<'info>] {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        self.to_vec()
    }

    fn to_account_metas(&self) -> Result<Vec<AccountMeta>, ProgramError> {
        // For raw account info slices, create simple account metas
        // preserving the original signer and writable flags
        Ok(self
            .iter()
            .map(|account| AccountMeta {
                pubkey: *account.key,
                is_signer: account.is_signer,
                is_writable: account.is_writable,
            })
            .collect())
    }

    fn get_mode(&self) -> Option<u8> {
        None // Unknown mode for raw slices
    }
}

// Implementation for CpiContextWriteAccounts
impl<'info> CpiAccountsTrait<'info> for CpiContextWriteAccounts<'info, AccountInfo<'info>> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.fee_payer.clone(),
            self.authority.clone(),
            self.cpi_context.clone(),
        ]
    }

    fn to_account_metas(&self) -> Result<Vec<AccountMeta>, ProgramError> {
        // Use the helper function to generate the account metas
        let metas = get_account_metas_from_config_cpi_context(self.clone());
        Ok(metas.to_vec())
    }

    fn get_mode(&self) -> Option<u8> {
        // CPI context write is a special case, typically used with mode 1 (small)
        Some(1)
    }
}

// Trait for Light CPI instruction types
pub trait LightCpiInstruction: Sized {
    fn new_cpi(cpi_signer: crate::cpi::CpiSigner, proof: ValidityProof) -> Self;
    fn with_light_account<A>(self, account: LightAccount<'_, A>) -> Result<Self, ProgramError>
    where
        A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + DataHasher + Default;
    fn get_mode(&self) -> u8;
    fn get_bump(&self) -> u8;
    #[cfg(feature = "v2")]
    fn write_to_cpi_context_first(self) -> Self;
    #[cfg(feature = "v2")]
    fn write_to_cpi_context_set(self) -> Self;
    #[cfg(feature = "v2")]
    fn execute_with_cpi_context(self) -> Self;
    #[cfg(feature = "v2")]
    fn get_with_cpi_context(&self) -> bool;
    #[cfg(feature = "v2")]
    fn get_cpi_context(
        &self,
    ) -> &light_compressed_account::instruction_data::cpi_context::CompressedCpiContext;
}

pub trait InvokeLightSystemProgram {
    fn invoke<'info>(self, accounts: impl CpiAccountsTrait<'info>) -> Result<(), ProgramError>;

    #[cfg(feature = "v2")]
    fn invoke_write_to_cpi_context_first<'info>(
        self,
        accounts: impl CpiAccountsTrait<'info>,
    ) -> Result<(), ProgramError>;

    #[cfg(feature = "v2")]
    fn invoke_write_to_cpi_context_set<'info>(
        self,
        accounts: impl CpiAccountsTrait<'info>,
    ) -> Result<(), ProgramError>;

    #[cfg(feature = "v2")]
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
        #[cfg(feature = "v2")]
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

    #[cfg(feature = "v2")]
    fn invoke_write_to_cpi_context_first<'info>(
        self,
        accounts: impl CpiAccountsTrait<'info>,
    ) -> Result<(), ProgramError> {
        let instruction_data = self.write_to_cpi_context_first();
        inner_invoke_write_to_cpi_context_typed(instruction_data, accounts)
    }

    #[cfg(feature = "v2")]
    fn invoke_write_to_cpi_context_set<'info>(
        self,
        accounts: impl CpiAccountsTrait<'info>,
    ) -> Result<(), ProgramError> {
        let instruction_data = self.write_to_cpi_context_set();
        inner_invoke_write_to_cpi_context_typed(instruction_data, accounts)
    }

    #[cfg(feature = "v2")]
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
#[cfg(feature = "v2")]
#[inline(always)]
pub fn inner_invoke_write_to_cpi_context_typed<'info, T>(
    instruction_data: T,
    accounts: impl CpiAccountsTrait<'info>,
) -> Result<(), ProgramError>
where
    T: LightInstructionData + LightCpiInstruction,
{
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

#[inline(always)]
pub fn invoke_light_system_program(
    account_infos: &[AccountInfo],
    instruction: Instruction,
    bump: u8,
) -> Result<(), ProgramError> {
    let signer_seeds = [CPI_AUTHORITY_PDA_SEED, &[bump]];
    invoke_signed(&instruction, account_infos, &[signer_seeds.as_slice()])
}
