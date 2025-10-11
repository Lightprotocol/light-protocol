use light_account_checks::AccountIterator;
use light_compressed_account::instruction_data::traits::AccountOptions;
use pinocchio::account_info::AccountInfo;

use crate::{
    accounts::{
        account_checks::{
            check_option_cpi_context_account, check_option_decompression_recipient,
            check_option_sol_pool_pda,
        },
        account_traits::{CpiContextAccountTrait, InvokeAccounts, SignerAccounts},
    },
    errors::SystemProgramError,
    Result,
};

#[derive(PartialEq, Eq)]
pub struct ExecutionAccounts<'info> {
    /// CHECK: in account compression program
    pub registered_program_pda: &'info AccountInfo,
    /// CHECK: used to invoke account compression program cpi sign will fail if invalid account is provided seeds = [CPI_AUTHORITY_PDA_SEED].
    pub account_compression_authority: &'info AccountInfo,
    pub account_compression_program: &'info AccountInfo,
    pub system_program: &'info AccountInfo,
    pub sol_pool_pda: Option<&'info AccountInfo>,
    /// CHECK: unchecked is user provided recipient.
    pub decompression_recipient: Option<&'info AccountInfo>,
}

#[derive(PartialEq, Eq)]
pub struct InvokeCpiInstructionV2<'info> {
    /// Fee payer needs to be mutable to pay rollover and protocol fees.
    pub fee_payer: &'info AccountInfo,
    pub authority: &'info AccountInfo,
    pub exec_accounts: Option<ExecutionAccounts<'info>>,
    pub cpi_context_account: Option<&'info AccountInfo>,
}

impl<'info> InvokeCpiInstructionV2<'info> {
    #[track_caller]
    pub fn from_account_infos(
        account_infos: &'info [AccountInfo],
        account_options: AccountOptions,
    ) -> Result<(Self, &'info [AccountInfo])> {
        let mut accounts = AccountIterator::new(account_infos);

        let fee_payer = accounts.next_signer_mut("fee_payer")?;
        let authority = accounts.next_signer_non_mut("authority")?;

        let exec_accounts = if !account_options.write_to_cpi_context {
            let registered_program_pda = accounts.next_non_mut("registered_program_pda")?;

            let account_compression_authority =
                accounts.next_non_mut("account_compression_authority")?;
            let account_compression_program =
                accounts.next_non_mut("account_compression_program")?;

            let system_program = accounts.next_non_mut("system_program")?;

            let sol_pool_pda = check_option_sol_pool_pda(&mut accounts, account_options)?;

            let decompression_recipient =
                check_option_decompression_recipient(&mut accounts, account_options)?;

            Some(ExecutionAccounts {
                registered_program_pda,
                account_compression_program,
                account_compression_authority,
                system_program,
                sol_pool_pda,
                decompression_recipient,
            })
        } else {
            None
        };

        let cpi_context_account = check_option_cpi_context_account(&mut accounts, account_options)?;
        let remaining_accounts = if !account_options.write_to_cpi_context {
            accounts.remaining()?
        } else {
            &[]
        };
        Ok((
            Self {
                fee_payer,
                authority,
                exec_accounts,
                cpi_context_account,
            },
            remaining_accounts,
        ))
    }
}

impl<'info> SignerAccounts<'info> for InvokeCpiInstructionV2<'info> {
    fn get_fee_payer(&self) -> &'info AccountInfo {
        self.fee_payer
    }

    fn get_authority(&self) -> &'info AccountInfo {
        self.authority
    }
}

impl<'info> CpiContextAccountTrait<'info> for InvokeCpiInstructionV2<'info> {
    fn get_cpi_context_account(&self) -> Option<&'info AccountInfo> {
        self.cpi_context_account
    }
}
impl<'info> InvokeAccounts<'info> for InvokeCpiInstructionV2<'info> {
    fn get_registered_program_pda(&self) -> Result<&'info AccountInfo> {
        self.exec_accounts
            .as_ref()
            .map(|exec| exec.registered_program_pda)
            .ok_or(SystemProgramError::CpiContextPassedAsSetContext.into())
    }

    fn get_account_compression_authority(&self) -> Result<&'info AccountInfo> {
        self.exec_accounts
            .as_ref()
            .map(|exec| exec.account_compression_authority)
            .ok_or(SystemProgramError::CpiContextPassedAsSetContext.into())
    }

    fn get_sol_pool_pda(&self) -> Result<Option<&'info AccountInfo>> {
        Ok(self
            .exec_accounts
            .as_ref()
            .and_then(|exec| exec.sol_pool_pda))
    }

    fn get_decompression_recipient(&self) -> Result<Option<&'info AccountInfo>> {
        Ok(self
            .exec_accounts
            .as_ref()
            .and_then(|exec| exec.decompression_recipient))
    }
}
