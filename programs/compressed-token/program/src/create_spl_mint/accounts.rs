use crate::shared::AccountIterator;
use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::checks::{check_mut, check_signer};
use pinocchio::account_info::AccountInfo;

pub struct CreateSplMintAccounts<'info> {
    pub fee_payer: &'info AccountInfo,
    pub authority: &'info AccountInfo,
    pub mint: &'info AccountInfo,
    pub mint_signer: &'info AccountInfo,
    pub token_pool_pda: &'info AccountInfo,
    pub token_program: &'info AccountInfo,
    pub cpi_authority_pda: &'info AccountInfo,
    pub light_system_program: &'info AccountInfo,
    pub registered_program_pda: &'info AccountInfo,
    pub noop_program: &'info AccountInfo,
    pub account_compression_authority: &'info AccountInfo,
    pub account_compression_program: &'info AccountInfo,
    pub system_program: &'info AccountInfo,
    pub self_program: &'info AccountInfo,
    pub in_merkle_tree: &'info AccountInfo,
    pub in_output_queue: &'info AccountInfo,
    pub out_output_queue: &'info AccountInfo,
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

        // CPI accounts in exact order expected by light-system-program
        let fee_payer = iter.next_account()?;
        let cpi_authority_pda = iter.next_account()?;
        let registered_program_pda = iter.next_account()?;
        let noop_program = iter.next_account()?;
        let account_compression_authority = iter.next_account()?;
        let account_compression_program = iter.next_account()?;
        let self_program = iter.next_account()?;

        let system_program = iter.next_account()?;
        let in_merkle_tree = iter.next_account()?;
        let in_output_queue = iter.next_account()?;
        let out_output_queue = iter.next_account()?;

        // Validate fee_payer: must be signer and mutable
        check_signer(fee_payer).map_err(ProgramError::from)?;
        check_mut(fee_payer).map_err(ProgramError::from)?;

        // Validate authority: must be signer
        check_signer(authority).map_err(ProgramError::from)?;

        Ok(CreateSplMintAccounts {
            fee_payer,
            authority,
            mint,
            mint_signer,
            token_pool_pda,
            token_program,
            cpi_authority_pda,
            light_system_program,
            registered_program_pda,
            noop_program,
            account_compression_authority,
            account_compression_program,
            system_program,
            self_program,
            in_merkle_tree,
            in_output_queue,
            out_output_queue,
        })
    }
}
