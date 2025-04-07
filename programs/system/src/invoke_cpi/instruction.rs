use light_account_checks::checks::{
    check_discriminator, check_owner, check_pda_seeds, check_program, check_signer,
};
use light_compressed_account::constants::ACCOUNT_COMPRESSION_PROGRAM_ID;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use super::account::CpiContextAccount;
use crate::{
    account_traits::{CpiContextAccountTrait, InvokeAccounts, SignerAccounts},
    processor::sol_compression::SOL_POOL_PDA_SEED,
    Result,
};

pub struct InvokeCpiInstruction<'info> {
    /// Fee payer needs to be mutable to pay rollover and protocol fees.
    pub fee_payer: &'info AccountInfo,
    pub authority: &'info AccountInfo,
    /// CHECK: in account compression program
    pub registered_program_pda: &'info AccountInfo,
    /// CHECK: checked in emit_event.rs.
    pub noop_program: &'info AccountInfo,
    /// CHECK: used to invoke account compression program cpi sign will fail if invalid account is provided seeds = [CPI_AUTHORITY_PDA_SEED].
    pub account_compression_authority: &'info AccountInfo,
    /// CHECK:
    pub account_compression_program: &'info AccountInfo,
    /// CHECK: checked in cpi_signer_check.
    pub invoking_program: &'info AccountInfo,
    pub sol_pool_pda: Option<&'info AccountInfo>,
    /// CHECK: unchecked is user provided recipient.
    pub decompression_recipient: Option<&'info AccountInfo>,
    pub system_program: &'info AccountInfo,
    pub cpi_context_account: Option<&'info AccountInfo>,
}

impl<'info> InvokeCpiInstruction<'info> {
    pub fn from_account_infos(
        accounts: &'info [AccountInfo],
    ) -> Result<(Self, &'info [AccountInfo])> {
        let fee_payer = &accounts[0];
        check_signer(fee_payer).map_err(ProgramError::from)?;
        let authority = &accounts[1];
        check_signer(authority).map_err(ProgramError::from)?;
        let registered_program_pda = &accounts[2];
        let noop_program = &accounts[3];
        let account_compression_authority = &accounts[4];
        let account_compression_program = &accounts[5];
        check_program(&ACCOUNT_COMPRESSION_PROGRAM_ID, account_compression_program)
            .map_err(ProgramError::from)?;
        let invoking_program = &accounts[6];
        let option_sol_pool_pda = &accounts[7];
        let sol_pool_pda = if *option_sol_pool_pda.key() == crate::ID {
            None
        } else {
            check_pda_seeds(&[SOL_POOL_PDA_SEED], &crate::ID, option_sol_pool_pda)
                .map_err(ProgramError::from)?;
            Some(option_sol_pool_pda)
        };
        let option_decompression_recipient = &accounts[8];
        let decompression_recipient = if *option_decompression_recipient.key() == crate::ID {
            None
        } else {
            Some(option_decompression_recipient)
        };
        let system_program = &accounts[9];
        check_program(&Pubkey::default(), system_program).map_err(ProgramError::from)?;
        let option_cpi_context_account = &accounts[10];

        let cpi_context_account = if *option_cpi_context_account.key() == crate::ID {
            None
        } else {
            check_owner(&crate::ID, option_cpi_context_account).map_err(ProgramError::from)?;
            check_discriminator::<CpiContextAccount, 8>(
                option_cpi_context_account.try_borrow_data()?.as_ref(),
            )
            .map_err(ProgramError::from)?;
            Some(option_cpi_context_account)
        };
        Ok((
            Self {
                fee_payer,
                authority,
                registered_program_pda,
                noop_program,
                account_compression_authority,
                account_compression_program,
                invoking_program,
                sol_pool_pda,
                decompression_recipient,
                system_program,
                cpi_context_account,
            },
            &accounts[11..],
        ))
    }
}

impl<'info> SignerAccounts<'info> for InvokeCpiInstruction<'info> {
    fn get_fee_payer(&self) -> &'info AccountInfo {
        self.fee_payer
    }

    fn get_authority(&self) -> &'info AccountInfo {
        self.authority
    }
}

impl<'info> CpiContextAccountTrait<'info> for InvokeCpiInstruction<'info> {
    fn get_cpi_context_account(&self) -> Option<&'info AccountInfo> {
        self.cpi_context_account
    }
}

impl<'info> InvokeAccounts<'info> for InvokeCpiInstruction<'info> {
    fn get_registered_program_pda(&self) -> &'info AccountInfo {
        self.registered_program_pda
    }

    fn get_account_compression_authority(&self) -> &'info AccountInfo {
        self.account_compression_authority
    }

    fn get_sol_pool_pda(&self) -> Option<&'info AccountInfo> {
        self.sol_pool_pda
    }

    fn get_decompression_recipient(&self) -> Option<&'info AccountInfo> {
        self.decompression_recipient
    }
}
