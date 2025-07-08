use crate::constants::BUMP_CPI_AUTHORITY;
use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::checks::{
    check_mut, check_non_mut, check_pda_seeds_with_bump, check_program, check_signer,
};
use light_compressed_account::constants::ACCOUNT_COMPRESSION_PROGRAM_ID;
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
    pub fn validate_and_parse(
        accounts: &'info [AccountInfo],
        program_id: &pinocchio::pubkey::Pubkey,
    ) -> Result<Self, ProgramError> {
        if accounts.len() < 17 {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        // Static non-CPI accounts first
        let authority = &accounts[0];
        let mint = &accounts[1];
        let mint_signer = &accounts[2];
        let token_pool_pda = &accounts[3];
        let token_program = &accounts[4];
        let light_system_program = &accounts[5];

        // CPI accounts in exact order expected by light-system-program
        let fee_payer = &accounts[6];
        let cpi_authority_pda = &accounts[7];
        let registered_program_pda = &accounts[8];
        let noop_program = &accounts[9];
        let account_compression_authority = &accounts[10];
        let account_compression_program = &accounts[11];
        let self_program = &accounts[12];

        let system_program = &accounts[13];
        let in_merkle_tree = &accounts[14];
        let in_output_queue = &accounts[15];
        let out_output_queue = &accounts[16];

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
