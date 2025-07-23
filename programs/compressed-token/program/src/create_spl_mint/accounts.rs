use std::ops::Deref;

use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::checks::{check_program, check_signer};
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};

use crate::shared::{
    accounts::{LightSystemAccounts, UpdateOneCompressedAccountTreeAccounts},
    AccountIterator,
};

pub struct CreateSplMintAccounts<'info> {
    pub authority: &'info AccountInfo,
    pub mint: &'info AccountInfo,
    pub mint_signer: &'info AccountInfo,
    pub token_pool_pda: &'info AccountInfo,
    pub token_program: &'info AccountInfo,
    pub light_system_program: &'info AccountInfo,
    pub system: LightSystemAccounts<'info>,
    pub trees: UpdateOneCompressedAccountTreeAccounts<'info>,
}

impl CreateSplMintAccounts<'_> {
    pub const SYSTEM_ACCOUNTS_OFFSET: usize = 6;
}

impl<'info> CreateSplMintAccounts<'info> {
    #[inline(always)]
    pub fn tree_pubkeys(&self) -> [&'info Pubkey; 3] {
        self.trees.pubkeys()
    }
}

impl<'info> Deref for CreateSplMintAccounts<'info> {
    type Target = LightSystemAccounts<'info>;

    fn deref(&self) -> &Self::Target {
        &self.system
    }
}

impl<'info> CreateSplMintAccounts<'info> {
    pub fn validate_and_parse(accounts: &'info [AccountInfo]) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);

        // Static non-CPI accounts first
        let authority = iter.next_account()?;
        let mint = iter.next_account()?;
        let mint_signer = iter.next_account()?;
        let token_pool_pda = iter.next_account()?;
        let token_program = iter.next_account()?;
        let light_system_program = iter.next_account()?;

        let system = LightSystemAccounts::validate_and_parse(&mut iter)?;
        let trees = UpdateOneCompressedAccountTreeAccounts::validate_and_parse(&mut iter)?;

        // Validate authority: must be signer
        check_signer(authority).map_err(ProgramError::from)?;

        check_program(&spl_token_2022::ID.to_bytes(), token_program).map_err(ProgramError::from)?;

        Ok(CreateSplMintAccounts {
            authority,
            mint,
            mint_signer,
            token_pool_pda,
            token_program,
            light_system_program,
            system,
            trees,
        })
    }
}
