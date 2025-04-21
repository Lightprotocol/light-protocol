use light_compressed_account::instruction_data::traits::AccountOptions;
use pinocchio::account_info::AccountInfo;

use crate::{
    accounts::{
        account_checks::{
            check_authority, check_fee_payer, check_non_mut_account_info,
            check_option_cpi_context_account, check_option_decompression_recipient,
            check_option_sol_pool_pda,
        },
        account_traits::{CpiContextAccountTrait, InvokeAccounts, SignerAccounts},
    },
    Result,
};

#[derive(PartialEq, Eq)]
pub struct InvokeCpiInstructionSmall<'info> {
    /// Fee payer needs to be mutable to pay rollover and protocol fees.
    pub fee_payer: &'info AccountInfo,
    pub authority: &'info AccountInfo,
    /// CHECK: in account compression program
    pub registered_program_pda: &'info AccountInfo,
    /// CHECK: used to invoke account compression program cpi sign will fail if invalid account is provided seeds = [CPI_AUTHORITY_PDA_SEED].
    pub account_compression_authority: &'info AccountInfo,
    pub sol_pool_pda: Option<&'info AccountInfo>,
    /// CHECK: unchecked is user provided recipient.
    pub decompression_recipient: Option<&'info AccountInfo>,
    pub cpi_context_account: Option<&'info AccountInfo>,
}

impl<'info> InvokeCpiInstructionSmall<'info> {
    pub fn from_account_infos(
        account_infos: &'info [AccountInfo],
        account_options: AccountOptions,
    ) -> Result<(Self, &'info [AccountInfo])> {
        let num_expected_static_accounts = 4 + account_options.get_num_expected_accounts();

        let (accounts, remaining_accounts) = account_infos.split_at(num_expected_static_accounts);

        let mut accounts = accounts.iter();
        let fee_payer = check_fee_payer(accounts.next())?;

        let authority = check_authority(accounts.next())?;

        let registered_program_pda = check_non_mut_account_info(accounts.next())?;

        let account_compression_authority = check_non_mut_account_info(accounts.next())?;

        let sol_pool_pda = check_option_sol_pool_pda(&mut accounts, account_options)?;

        let decompression_recipient =
            check_option_decompression_recipient(&mut accounts, account_options)?;

        let cpi_context_account = check_option_cpi_context_account(&mut accounts, account_options)?;
        assert!(accounts.next().is_none());

        Ok((
            Self {
                fee_payer,
                authority,
                registered_program_pda,
                account_compression_authority,
                sol_pool_pda,
                decompression_recipient,
                cpi_context_account,
            },
            remaining_accounts,
        ))
    }
}

impl<'info> SignerAccounts<'info> for InvokeCpiInstructionSmall<'info> {
    fn get_fee_payer(&self) -> &'info AccountInfo {
        self.fee_payer
    }

    fn get_authority(&self) -> &'info AccountInfo {
        self.authority
    }
}

impl<'info> CpiContextAccountTrait<'info> for InvokeCpiInstructionSmall<'info> {
    fn get_cpi_context_account(&self) -> Option<&'info AccountInfo> {
        self.cpi_context_account
    }
}
impl<'info> InvokeAccounts<'info> for InvokeCpiInstructionSmall<'info> {
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
