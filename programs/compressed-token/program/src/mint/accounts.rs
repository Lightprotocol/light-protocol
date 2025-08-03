use anchor_lang::solana_program::program_error::ProgramError;
use pinocchio::{account_info::AccountInfo, log::sol_log_compute_units, msg, pubkey::Pubkey};

use crate::shared::{
    accounts::{
        CpiContextLightSystemAccounts, CreateCompressedAccountTreeAccounts, LightSystemAccounts,
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
    #[inline(always)]
    pub fn validate_and_parse(
        accounts: &'info [AccountInfo],
        with_cpi_context: bool,
        write_to_cpi_context: bool,
    ) -> Result<Self, ProgramError> {
        // 1 CU
        let mut iter = AccountIterator::new(accounts);

        // Static non-CPI accounts first
        // 9 CU
        let mint_signer = iter.next_signer("mint_signer")?;
        // 18 CU
        let light_system_program = iter.next_non_mut("light_system_program")?;
        if write_to_cpi_context {
            // 46 CU
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
