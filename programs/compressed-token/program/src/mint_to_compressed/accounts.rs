use std::ops::Deref;

use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::checks::check_signer;
use pinocchio::account_info::AccountInfo;

use crate::shared::{accounts::LightSystemAccounts, AccountIterator};

pub struct MintToCompressedAccounts<'info> {
    pub authority: &'info AccountInfo,
    pub mint: Option<&'info AccountInfo>,
    pub token_pool_pda: Option<&'info AccountInfo>,
    pub token_program: Option<&'info AccountInfo>,
    pub light_system_program: &'info AccountInfo,
    pub system: LightSystemAccounts<'info>,
    pub sol_pool_pda: Option<&'info AccountInfo>,
    pub mint_in_merkle_tree: &'info AccountInfo,
    pub mint_in_queue: &'info AccountInfo,
    pub mint_out_queue: &'info AccountInfo,
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
        // Calculate minimum accounts needed
        let mut base_accounts = 13;

        if with_lamports {
            base_accounts += 1;
        };
        if is_decompressed {
            base_accounts += 3; // Add mint, token_pool_pda, token_program
        };
        if accounts.len() < base_accounts {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        let mut iter = AccountIterator::new(accounts);

        // Static non-CPI accounts first
        let authority = iter.next_account()?;

        let (mint, token_pool_pda, token_program) = if is_decompressed {
            (
                Some(iter.next_account()?),
                Some(iter.next_account()?),
                Some(iter.next_account()?),
            )
        } else {
            (None, None, None)
        };

        let light_system_program = iter.next_account()?;

        let system = LightSystemAccounts::validate_and_parse(&mut iter)?;

        let sol_pool_pda = if with_lamports {
            Some(iter.next_account()?)
        } else {
            None
        };

        let mint_in_merkle_tree = iter.next_account()?;
        let mint_in_queue = iter.next_account()?;
        let mint_out_queue = iter.next_account()?;
        let tokens_out_queue = iter.next_account()?;

        // Validate authority: must be signer
        check_signer(authority).map_err(ProgramError::from)?;

        Ok(MintToCompressedAccounts {
            authority,
            mint,
            token_pool_pda,
            token_program,
            light_system_program,
            system,
            sol_pool_pda,
            mint_in_merkle_tree,
            mint_in_queue,
            mint_out_queue,
            tokens_out_queue,
        })
    }
}
