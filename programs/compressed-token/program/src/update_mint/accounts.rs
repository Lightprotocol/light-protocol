use anchor_lang::solana_program::program_error::ProgramError;
use pinocchio::account_info::AccountInfo;

use crate::shared::{
    accounts::{
        CpiContextLightSystemAccounts, LightSystemAccounts, UpdateOneCompressedAccountTreeAccounts,
    },
    AccountIterator,
};

pub struct UpdateCompressedMintAccounts<'info> {
    pub light_system_program: &'info AccountInfo,
    pub authority: &'info AccountInfo,
    pub executing: Option<ExecutingAccounts<'info>>,
    pub write_to_cpi_context_system: Option<CpiContextLightSystemAccounts<'info>>,
}

pub struct ExecutingAccounts<'info> {
    pub system: LightSystemAccounts<'info>,
    pub tree_accounts: UpdateOneCompressedAccountTreeAccounts<'info>,
}

impl<'info> UpdateCompressedMintAccounts<'info> {
    pub fn validate_and_parse(
        accounts: &'info [AccountInfo],
        with_cpi_context: bool,
        write_to_cpi_context: bool,
    ) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);
        let light_system_program = iter.next_account("light_system_program")?;
        let authority = iter.next_signer("authority")?;
        
        if write_to_cpi_context {
            Ok(UpdateCompressedMintAccounts {
                light_system_program,
                authority,
                executing: None,
                write_to_cpi_context_system: Some(
                    CpiContextLightSystemAccounts::validate_and_parse(&mut iter)?,
                ),
            })
        } else {
            let system = LightSystemAccounts::validate_and_parse(
                &mut iter,
                false, // no lamports for update mint
                false, // no decompression
                with_cpi_context,
            )?;

            let tree_accounts =
                UpdateOneCompressedAccountTreeAccounts::validate_and_parse(&mut iter)?;

            Ok(UpdateCompressedMintAccounts {
                light_system_program,
                authority,
                executing: Some(ExecutingAccounts {
                    system,
                    tree_accounts,
                }),
                write_to_cpi_context_system: None,
            })
        }
    }

    pub fn cpi_authority(&self) -> Result<&AccountInfo, ProgramError> {
        if let Some(executing) = &self.executing {
            Ok(executing.system.cpi_authority_pda)
        } else {
            let cpi_system = self
                .write_to_cpi_context_system
                .as_ref()
                .ok_or(ProgramError::InvalidInstructionData)?;
            Ok(cpi_system.cpi_authority_pda)
        }
    }
}