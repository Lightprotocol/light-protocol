use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

use crate::{
    accounts::{
        account_checks::{
            anchor_option_mut_account_info, check_account_compression_program,
            check_anchor_option_cpi_context_account, check_anchor_option_sol_pool_pda,
            check_authority, check_fee_payer, check_non_mut_account_info, check_system_program,
        },
        account_traits::{CpiContextAccountTrait, InvokeAccounts, SignerAccounts},
    },
    Result,
};

#[derive(PartialEq, Eq)]
pub struct InvokeCpiInstruction<'info> {
    /// Fee payer needs to be mutable to pay rollover and protocol fees.
    pub fee_payer: &'info AccountInfo,
    /// CHECK: is non mutable.
    pub authority: &'info AccountInfo,
    /// CHECK: is non mutable.
    pub registered_program_pda: &'info AccountInfo,
    /// CHECK: used to invoke account compression program cpi sign will fail if invalid account is provided seeds = [CPI_AUTHORITY_PDA_SEED].
    pub account_compression_authority: &'info AccountInfo,
    /// CHECK: program id and is executable.
    pub account_compression_program: &'info AccountInfo,
    /// CHECK: checked in cpi_signer_check.
    pub invoking_program: &'info AccountInfo,
    /// CHECK: derivation.
    pub sol_pool_pda: Option<&'info AccountInfo>,
    /// CHECK: unchecked is user provided recipient.
    pub decompression_recipient: Option<&'info AccountInfo>,
    /// CHECK: is system program.
    pub system_program: &'info AccountInfo,
    /// CHECK: owner, discriminator, and association
    ///     with first state Merkle tree (the latter during processing).
    pub cpi_context_account: Option<&'info AccountInfo>,
}

impl<'info> InvokeCpiInstruction<'info> {
    pub fn from_account_infos(
        account_infos: &'info [AccountInfo],
    ) -> Result<(Self, &'info [AccountInfo])> {
        let (accounts, remaining_accounts) = account_infos.split_at(11);
        let mut accounts = accounts.iter();
        let fee_payer = check_fee_payer(accounts.next())?;

        let authority = check_authority(accounts.next())?;
        let registered_program_pda = check_non_mut_account_info(accounts.next())?;
        let _noop_program = accounts.next().ok_or(ProgramError::NotEnoughAccountKeys)?;
        let account_compression_authority = check_non_mut_account_info(accounts.next())?;

        let account_compression_program = check_account_compression_program(accounts.next())?;

        let invoking_program = accounts.next().ok_or(ProgramError::NotEnoughAccountKeys)?;

        let sol_pool_pda = check_anchor_option_sol_pool_pda(accounts.next())?;

        let decompression_recipient = anchor_option_mut_account_info(accounts.next())?;

        let system_program = check_system_program(accounts.next())?;

        let cpi_context_account = check_anchor_option_cpi_context_account(accounts.next())?;
        assert!(accounts.next().is_none());

        Ok((
            Self {
                fee_payer,
                authority,
                registered_program_pda,
                account_compression_authority,
                account_compression_program,
                invoking_program,
                sol_pool_pda,
                decompression_recipient,
                system_program,
                cpi_context_account,
            },
            remaining_accounts,
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
    fn get_registered_program_pda(&self) -> Result<&'info AccountInfo> {
        Ok(self.registered_program_pda)
    }

    fn get_account_compression_authority(&self) -> Result<&'info AccountInfo> {
        Ok(self.account_compression_authority)
    }

    fn get_sol_pool_pda(&self) -> Result<Option<&'info AccountInfo>> {
        Ok(self.sol_pool_pda)
    }

    fn get_decompression_recipient(&self) -> Result<Option<&'info AccountInfo>> {
        Ok(self.decompression_recipient)
    }
}
