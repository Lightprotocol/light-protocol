use std::ops::Deref;

use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::checks::check_signer;
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};

use crate::shared::{
    accounts::{CreateCompressedAccountTreeAccounts, LightSystemAccounts},
    AccountIterator,
};

pub struct CreateCompressedMintAccounts<'info> {
    pub mint_signer: &'info AccountInfo,
    pub light_system_program: &'info AccountInfo,
    pub system: LightSystemAccounts<'info>,
    pub trees: CreateCompressedAccountTreeAccounts<'info>,
}

impl<'info> Deref for CreateCompressedMintAccounts<'info> {
    type Target = LightSystemAccounts<'info>;

    fn deref(&self) -> &Self::Target {
        &self.system
    }
}

impl CreateCompressedMintAccounts<'_> {
    pub const CPI_ACCOUNTS_OFFSET: usize = 2;
}

impl<'info> CreateCompressedMintAccounts<'info> {
    pub fn validate_and_parse(accounts: &'info [AccountInfo]) -> Result<Self, ProgramError> {
        if accounts.len() != 12 {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        let mut iter = AccountIterator::new(accounts);

        // Static non-CPI accounts first
        let mint_signer = iter.next_account()?;
        let light_system_program = iter.next_account()?;

        let system = LightSystemAccounts::validate_and_parse(&mut iter)?;

        let trees = CreateCompressedAccountTreeAccounts::validate_and_parse(&mut iter)?;

        // Validate mint_signer: must be signer
        check_signer(mint_signer).map_err(ProgramError::from)?;

        Ok(CreateCompressedMintAccounts {
            mint_signer,
            light_system_program,
            system,
            trees,
        })
    }

    #[inline(always)]
    pub fn tree_pubkeys(&self) -> [&'info Pubkey; 2] {
        self.trees.pubkeys()
    }
}
