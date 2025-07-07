use crate::constants::BUMP_CPI_AUTHORITY;
use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::checks::{
    check_mut, check_non_mut, check_pda_seeds_with_bump, check_program, check_signer,
};
use light_compressed_account::constants::ACCOUNT_COMPRESSION_PROGRAM_ID;
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};

pub struct MintToCompressedAccounts<'info> {
    pub fee_payer: &'info AccountInfo,
    pub authority: &'info AccountInfo,
    pub cpi_authority_pda: &'info AccountInfo,
    pub mint: Option<&'info AccountInfo>,
    pub token_pool_pda: &'info AccountInfo,
    pub token_program: &'info AccountInfo,
    pub light_system_program: &'info AccountInfo,
    pub registered_program_pda: &'info AccountInfo,
    pub noop_program: &'info AccountInfo,
    pub account_compression_authority: &'info AccountInfo,
    pub account_compression_program: &'info AccountInfo,
    pub self_program: &'info AccountInfo,
    pub system_program: &'info AccountInfo,
    pub sol_pool_pda: Option<&'info AccountInfo>,
    pub mint_in_merkle_tree: &'info AccountInfo,
    pub mint_in_queue: &'info AccountInfo,
    pub mint_out_queue: &'info AccountInfo,
    pub tokens_out_queue: &'info AccountInfo,
}

impl<'info> MintToCompressedAccounts<'info> {
    pub fn validate_and_parse(
        accounts: &'info [AccountInfo],
        program_id: &Pubkey,
        with_lamports: bool,
    ) -> Result<Self, ProgramError> {
        if accounts.len() < 18 {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        let fee_payer = &accounts[0];
        let authority = &accounts[1];
        let cpi_authority_pda = &accounts[2];
        let mint = if accounts.len() > 14 && accounts[3].data_is_empty() {
            None
        } else {
            Some(&accounts[3])
        };
        let token_pool_pda = &accounts[4];
        let token_program = &accounts[5];
        let light_system_program = &accounts[6];
        let registered_program_pda = &accounts[7];
        let noop_program = &accounts[8];
        let account_compression_authority = &accounts[9];
        let account_compression_program = &accounts[10];
        let self_program = &accounts[11];
        let system_program = &accounts[12];
        let mut index = 13;
        let sol_pool_pda = if with_lamports {
            index += 1;
            Some(&accounts[index])
        } else {
            None
        };
        let mint_in_merkle_tree = &accounts[index];
        let mint_in_queue = &accounts[index + 1];
        let mint_out_queue = &accounts[index + 1];
        let tokens_out_queue = &accounts[index + 1];

        // Validate fee_payer: must be signer and mutable
        check_signer(fee_payer).map_err(ProgramError::from)?;
        check_mut(fee_payer).map_err(ProgramError::from)?;

        // Validate authority: must be signer
        check_signer(authority).map_err(ProgramError::from)?;

        // Validate cpi_authority_pda: must be the correct PDA
        let expected_seeds = &[CPI_AUTHORITY_PDA_SEED, &[BUMP_CPI_AUTHORITY]];
        check_pda_seeds_with_bump(expected_seeds, &program_id, cpi_authority_pda)
            .map_err(ProgramError::from)?;

        // Validate mint: mutable if present
        if let Some(mint_account) = mint {
            check_mut(mint_account).map_err(ProgramError::from)?;
        }

        // Validate token_pool_pda: mutable
        check_mut(token_pool_pda).map_err(ProgramError::from)?;

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

        // Validate self_program: must be this program
        check_program(&program_id, self_program).map_err(ProgramError::from)?;

        // Validate system_program: must be the system program
        let system_program_id = anchor_lang::solana_program::system_program::ID;
        check_program(&system_program_id.to_bytes(), system_program).map_err(ProgramError::from)?;

        // Validate sol_pool_pda: mutable if present
        if let Some(sol_pool_account) = sol_pool_pda {
            check_mut(sol_pool_account).map_err(ProgramError::from)?;
        }

        // Validate merkle_tree: mutable
        check_mut(mint_in_merkle_tree).map_err(ProgramError::from)?;
        // Validate merkle_tree: mutable
        check_mut(mint_in_queue).map_err(ProgramError::from)?;
        // Validate merkle_tree: mutable
        check_mut(mint_out_queue).map_err(ProgramError::from)?;
        // Validate merkle_tree: mutable
        check_mut(tokens_out_queue).map_err(ProgramError::from)?;

        Ok(MintToCompressedAccounts {
            fee_payer,
            authority,
            cpi_authority_pda,
            mint,
            token_pool_pda,
            token_program,
            light_system_program,
            registered_program_pda,
            noop_program,
            account_compression_authority,
            account_compression_program,
            self_program,
            system_program,
            sol_pool_pda,
            mint_in_merkle_tree,
            mint_in_queue,
            mint_out_queue,
            tokens_out_queue,
        })
    }
}
