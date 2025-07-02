use light_account_checks::checks::check_signer;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

use crate::{
    accounts::{
        account_checks::{
            anchor_option_mut_account_info, check_account_compression_program,
            check_anchor_option_sol_pool_pda, check_fee_payer, check_non_mut_account_info,
            check_system_program,
        },
        account_traits::{InvokeAccounts, SignerAccounts},
    },
    Result,
};

/// These are the base accounts additionally Merkle tree and queue accounts are required.
/// These additional accounts are passed as remaining accounts.
/// 1 Merkle tree for each input compressed account one queue and Merkle tree account each for each output compressed account.
#[derive(PartialEq, Eq)]
pub struct InvokeInstruction<'info> {
    /// Fee payer needs to be mutable to pay rollover and protocol fees.
    pub fee_payer: &'info AccountInfo,
    pub authority: &'info AccountInfo,
    /// CHECK: in account compression program.
    pub registered_program_pda: &'info AccountInfo,
    /// CHECK: this account in account compression program.
    /// This pda is used to invoke the account compression program.
    pub account_compression_authority: &'info AccountInfo,
    /// CHECK: Account compression program is used to update state and address
    /// Merkle trees.
    pub account_compression_program: &'info AccountInfo,
    /// Sol pool pda is used to store the native sol that has been compressed.
    /// It's only required when compressing or decompressing sol.
    pub sol_pool_pda: Option<&'info AccountInfo>,
    /// Unchecked recipient for decompressed sol.
    /// Compressed sol originate from sol_pool_pda.
    pub decompression_recipient: Option<&'info AccountInfo>,
    pub system_program: &'info AccountInfo,
}

impl<'info> InvokeInstruction<'info> {
    pub fn from_account_infos(
        account_infos: &'info [AccountInfo],
    ) -> Result<(Self, &'info [AccountInfo])> {
        let (accounts, remaining_accounts) = account_infos.split_at(9);
        let mut accounts = accounts.iter();
        let fee_payer = check_fee_payer(accounts.next())?;

        // Fee payer and authority can be the same account in case of invoke.
        let authority = accounts.next().ok_or(ProgramError::NotEnoughAccountKeys)?;
        check_signer(authority).map_err(ProgramError::from)?;

        let registered_program_pda = check_non_mut_account_info(accounts.next())?;

        // For backwards compatibility we skip an account
        // in this index previously the noop program was passed here.
        let _unused_account_info = accounts.next().ok_or(ProgramError::NotEnoughAccountKeys)?;

        let account_compression_authority = check_non_mut_account_info(accounts.next())?;

        let account_compression_program = check_account_compression_program(accounts.next())?;

        let sol_pool_pda = check_anchor_option_sol_pool_pda(accounts.next())?;

        let decompression_recipient = anchor_option_mut_account_info(accounts.next())?;

        let system_program = check_system_program(accounts.next())?;

        assert!(accounts.next().is_none());

        Ok((
            Self {
                fee_payer,
                authority,
                registered_program_pda,
                account_compression_authority,
                account_compression_program,
                sol_pool_pda,
                decompression_recipient,
                system_program,
            },
            remaining_accounts,
        ))
    }
}

impl<'info> SignerAccounts<'info> for InvokeInstruction<'info> {
    fn get_fee_payer(&self) -> &'info AccountInfo {
        self.fee_payer
    }

    fn get_authority(&self) -> &'info AccountInfo {
        self.authority
    }
}

impl<'info> InvokeAccounts<'info> for InvokeInstruction<'info> {
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
