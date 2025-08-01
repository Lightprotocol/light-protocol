use anchor_lang::solana_program::program_error::ProgramError;
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};

use crate::shared::{
    accounts::{
        CpiContextLightSystemAccounts, CreateCompressedAccountTreeAccounts, LightSystemAccounts,
        UpdateOneCompressedAccountTreeAccounts,
    },
    AccountIterator,
};

pub struct CreateCompressedMintAccounts<'info> {
    pub mint_signer: &'info AccountInfo,
    pub light_system_program: &'info AccountInfo,
    pub system: Option<LightSystemAccounts<'info>>,
    pub trees: Option<CreateCompressedAccountTreeAccounts<'info>>,
    pub cpi_context_light_system_accounts: Option<CpiContextLightSystemAccounts<'info>>,
}

impl CreateCompressedMintAccounts<'_> {
    pub const CPI_ACCOUNTS_OFFSET: usize = 2;
}

impl<'info> CreateCompressedMintAccounts<'info> {
    pub fn validate_and_parse(
        accounts: &'info [AccountInfo],
        with_cpi_context: bool,
        write_to_cpi_context: bool,
    ) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);

        // Static non-CPI accounts first
        let mint_signer = iter.next_signer("mint_signer")?;
        let light_system_program = iter.next_non_mut("light_system_program")?;
        if write_to_cpi_context {
            let cpi_context_light_system_accounts =
                CpiContextLightSystemAccounts::validate_and_parse(&mut iter)?;

            Ok(CreateCompressedMintAccounts {
                mint_signer,
                light_system_program,
                system: None,
                trees: None,
                cpi_context_light_system_accounts: Some(cpi_context_light_system_accounts),
            })
        } else {
            let system =
                LightSystemAccounts::validate_and_parse(&mut iter, false, false, with_cpi_context)?;

            let trees = CreateCompressedAccountTreeAccounts::validate_and_parse(&mut iter)?;

            Ok(CreateCompressedMintAccounts {
                mint_signer,
                light_system_program,
                system: Some(system),
                trees: Some(trees),
                cpi_context_light_system_accounts: None,
            })
        }
    }

    #[inline(always)]
    pub fn tree_pubkeys(&self) -> Option<[&'info Pubkey; 2]> {
        if let Some(trees) = self.trees.as_ref() {
            Some(trees.pubkeys())
        } else {
            None
        }
    }
}
