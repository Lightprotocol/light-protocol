use crate::constants::BUMP_CPI_AUTHORITY;
use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::solana_program::program_error::ProgramError;
use pinocchio::account_info::AccountInfo;
use light_account_checks::checks::{
    check_mut, check_non_mut, check_pda_seeds_with_bump, check_program, check_signer,
};
use light_compressed_account::constants::ACCOUNT_COMPRESSION_PROGRAM_ID;

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
        
        // CPI accounts in exact order expected by light-system-program
        let fee_payer = &accounts[5];
        let cpi_authority_pda = &accounts[6];
        let registered_program_pda = &accounts[7];
        let noop_program = &accounts[8];
        let account_compression_authority = &accounts[9];
        let account_compression_program = &accounts[10];
        let self_program = &accounts[11];
        let light_system_program = &accounts[12];
        let system_program = &accounts[13];
        let in_merkle_tree = &accounts[14];
        let in_output_queue = &accounts[15];
        let out_output_queue = &accounts[16];

        // Validate fee_payer: must be signer and mutable
        check_signer(fee_payer).map_err(ProgramError::from)?;
        check_mut(fee_payer).map_err(ProgramError::from)?;

        // Validate authority: must be signer
        check_signer(authority).map_err(ProgramError::from)?;

        // Validate mint: must be mutable (will be created in instruction)
        check_mut(mint).map_err(ProgramError::from)?;

        // mint_signer: no specific validation (unchecked account)

        // Validate token_pool_pda: must be mutable (will be created in instruction)
        check_mut(token_pool_pda).map_err(ProgramError::from)?;

        // Validate token_program: must be the Token2022 program
        let token_2022_program_id = spl_token_2022::id();
        check_program(&token_2022_program_id.to_bytes(), token_program)
            .map_err(ProgramError::from)?;

        // Validate cpi_authority_pda: must be the correct PDA
        let expected_seeds = &[CPI_AUTHORITY_PDA_SEED, &[BUMP_CPI_AUTHORITY]];
        check_pda_seeds_with_bump(expected_seeds, program_id, cpi_authority_pda)
            .map_err(ProgramError::from)?;

        // Validate light_system_program: must be the correct program
        let light_system_program_id = light_system_program::id();
        check_program(&light_system_program_id.to_bytes(), light_system_program)
            .map_err(ProgramError::from)?;

        // Validate registered_program_pda: non-mutable
        check_non_mut(registered_program_pda).map_err(ProgramError::from)?;

        // Validate noop_program: non-mutable
        check_non_mut(noop_program).map_err(ProgramError::from)?;

        // Validate account_compression_authority: non-mutable
        check_non_mut(account_compression_authority).map_err(ProgramError::from)?;

        // Validate account_compression_program: must be the correct program
        check_program(&ACCOUNT_COMPRESSION_PROGRAM_ID, account_compression_program)
            .map_err(ProgramError::from)?;

        // Validate system_program: must be the system program
        let system_program_id = anchor_lang::solana_program::system_program::ID;
        check_program(&system_program_id.to_bytes(), system_program)
            .map_err(ProgramError::from)?;

        // Validate self_program: must be this program
        check_program(program_id, self_program).map_err(ProgramError::from)?;

        // Validate in_merkle_tree: mutable
        check_mut(in_merkle_tree).map_err(ProgramError::from)?;

        // Validate in_output_queue: mutable
        check_mut(in_output_queue).map_err(ProgramError::from)?;

        // Validate out_output_queue: mutable
        check_mut(out_output_queue).map_err(ProgramError::from)?;

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