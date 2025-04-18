use light_account_checks::checks::{
    check_discriminator, check_non_mut, check_owner, check_pda_seeds, check_signer,
};
use light_compressed_account::instruction_data::traits::AccountOptions;
use pinocchio::{account_info::AccountInfo, msg};

use crate::{
    accounts::account_traits::{CpiContextAccountTrait, InvokeAccounts, SignerAccounts},
    invoke_cpi::account::CpiContextAccount,
    processor::sol_compression::SOL_POOL_PDA_SEED,
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
        accounts: &'info [AccountInfo],
        options_config: AccountOptions,
    ) -> Result<(Self, &'info [AccountInfo])> {
        let fee_payer = &accounts[0];
        check_signer(fee_payer)?;

        let authority = &accounts[1];
        check_signer(authority)?;
        check_non_mut(authority)?;

        let registered_program_pda = &accounts[2];
        check_non_mut(registered_program_pda)?;

        let account_compression_authority = &accounts[3];
        check_non_mut(account_compression_authority)?;
        msg!(format!("options_config {:?}", options_config).as_str());

        msg!("here");
        let mut account_counter = 4;
        let sol_pool_pda = if options_config.sol_pool_pda {
            let option_sol_pool_pda = &accounts[account_counter];
            check_pda_seeds(&[SOL_POOL_PDA_SEED], &crate::ID, option_sol_pool_pda)?;
            account_counter += 1;
            Some(option_sol_pool_pda)
        } else {
            None
        };

        let decompression_recipient = if options_config.decompression_recipient {
            let option_decompression_recipient = &accounts[account_counter];
            account_counter += 1;
            Some(option_decompression_recipient)
        } else {
            None
        };

        let cpi_context_account = if options_config.cpi_context_account {
            let option_cpi_context_account = &accounts[account_counter];
            check_owner(&crate::ID, option_cpi_context_account)?;
            check_discriminator::<CpiContextAccount>(
                option_cpi_context_account.try_borrow_data()?.as_ref(),
            )?;
            Some(option_cpi_context_account)
        } else {
            None
        };

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
            &accounts[account_counter..],
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
