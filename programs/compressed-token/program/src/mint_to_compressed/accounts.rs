use anchor_lang::solana_program::program_error::ProgramError;
use pinocchio::{account_info::AccountInfo, msg};

use crate::shared::{
    accounts::{
        CpiContextLightSystemAccounts, LightSystemAccounts, UpdateOneCompressedAccountTreeAccounts,
    },
    AccountIterator,
};

pub struct MintToCompressedAccounts<'info> {
    pub light_system_program: &'info AccountInfo,
    pub authority: &'info AccountInfo,
    pub executing: Option<ExecutingAccounts<'info>>,
    pub write_to_cpi_context_system: Option<CpiContextLightSystemAccounts<'info>>,
}

pub struct ExecutingAccounts<'info> {
    pub mint: Option<&'info AccountInfo>,
    pub token_pool_pda: Option<&'info AccountInfo>,
    pub token_program: Option<&'info AccountInfo>,
    pub system: LightSystemAccounts<'info>,
    pub tree_accounts: UpdateOneCompressedAccountTreeAccounts<'info>,
    pub tokens_out_queue: &'info AccountInfo,
}

impl<'info> MintToCompressedAccounts<'info> {
    pub fn validate_and_parse(
        accounts: &'info [AccountInfo],
        with_lamports: bool,
        is_decompressed: bool,
        with_cpi_context: bool,
        write_to_cpi_context: bool,
    ) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);
        let light_system_program = iter.next_account("light_system_program")?;
        // Static non-CPI accounts first
        let authority = iter.next_signer("authority")?;
        if write_to_cpi_context {
            msg!("write to cpi context");
            Ok(MintToCompressedAccounts {
                light_system_program,
                authority,
                executing: None,
                write_to_cpi_context_system: Some(
                    CpiContextLightSystemAccounts::validate_and_parse(&mut iter)?,
                ),
            })
        } else {
            let mint = iter.next_option_mut("mint", is_decompressed)?;
            let token_pool_pda = iter.next_option_mut("token_pool_pda", is_decompressed)?;
            let token_program = iter.next_option("token_program", is_decompressed)?;

            let system = LightSystemAccounts::validate_and_parse(
                &mut iter,
                with_lamports,
                false,
                with_cpi_context,
            )?;

            let tree_accounts =
                UpdateOneCompressedAccountTreeAccounts::validate_and_parse(&mut iter)?;
            let tokens_out_queue = iter.next_account("tokens_out_queue")?;

            Ok(MintToCompressedAccounts {
                light_system_program,
                authority,
                executing: Some(ExecutingAccounts {
                    mint,
                    token_pool_pda,
                    token_program,
                    system,
                    tree_accounts,
                    tokens_out_queue,
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
                .ok_or(ProgramError::InvalidInstructionData)?; // TODO: better error
            Ok(cpi_system.cpi_authority_pda)
        }
    }
}
