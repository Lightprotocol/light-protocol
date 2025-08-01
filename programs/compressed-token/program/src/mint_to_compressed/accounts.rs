use std::ops::Deref;

use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::checks::check_signer;
use pinocchio::account_info::AccountInfo;

use crate::shared::{
    accounts::{LightSystemAccounts, UpdateOneCompressedAccountTreeAccounts},
    AccountIterator,
};

pub struct MintToCompressedAccounts<'info> {
    pub authority: &'info AccountInfo,
    pub mint: Option<&'info AccountInfo>,
    pub token_pool_pda: Option<&'info AccountInfo>,
    pub token_program: Option<&'info AccountInfo>,
    pub light_system_program: &'info AccountInfo,
    pub system: LightSystemAccounts<'info>,
    pub sol_pool_pda: Option<&'info AccountInfo>,
    pub tree_accounts: UpdateOneCompressedAccountTreeAccounts<'info>,
    pub tokens_out_queue: &'info AccountInfo,
}

impl<'info> Deref for MintToCompressedAccounts<'info> {
    type Target = LightSystemAccounts<'info>;

    fn deref(&self) -> &Self::Target {
        &self.system
    }
}

impl<'info> MintToCompressedAccounts<'info> {
    pub fn validate_and_parse(
        accounts: &'info [AccountInfo],
        with_lamports: bool,
        is_decompressed: bool,
    ) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);

        // Static non-CPI accounts first
        let authority = iter.next_account("authority")?;

        let (mint, token_pool_pda, token_program) = if is_decompressed {
            (
                Some(iter.next_account("mint")?),
                Some(iter.next_account("token_pool_pda")?),
                Some(iter.next_account("token_program")?),
            )
        } else {
            (None, None, None)
        };

        let light_system_program = iter.next_account("light_system_program")?;

        let system = LightSystemAccounts::validate_and_parse(&mut iter)?;

        let sol_pool_pda = if with_lamports {
            Some(iter.next_account("sol_pool_pda")?)
        } else {
            None
        };

        let tree_accounts = UpdateOneCompressedAccountTreeAccounts::validate_and_parse(&mut iter)?;
        let tokens_out_queue = iter.next_account("tokens_out_queue")?;

        // Validate authority: must be signer
        check_signer(authority)?;

        Ok(MintToCompressedAccounts {
            authority,
            mint,
            token_pool_pda,
            token_program,
            light_system_program,
            system,
            sol_pool_pda,
            tree_accounts,
            tokens_out_queue,
        })
    }
}
