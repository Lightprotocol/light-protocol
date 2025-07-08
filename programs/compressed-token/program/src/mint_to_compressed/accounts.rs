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
    pub token_pool_pda: Option<&'info AccountInfo>,
    pub token_program: Option<&'info AccountInfo>,
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
        anchor_lang::solana_program::msg!(
            "account len {} is less than required {}",
            accounts.len(),
            base_accounts
        );
        if accounts.len() < base_accounts {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        // Static non-CPI accounts first
        let authority = &accounts[0];
        let mut index = 1;
        let (mint, token_pool_pda, token_program) = if is_decompressed {
            let mint = Some(&accounts[index]);
            index += 1;
            let token_pool_pda = Some(&accounts[index]);
            index += 1;
            let token_program = Some(&accounts[index]);
            index += 1;
            (mint, token_pool_pda, token_program)
        } else {
            (None, None, None)
        };

        let light_system_program = &accounts[index];
        index += 1;
        // CPI accounts in exact order expected by InvokeCpiWithReadOnly
        let fee_payer = &accounts[index];
        index += 1;
        let cpi_authority_pda = &accounts[index];
        index += 1;
        let registered_program_pda = &accounts[index];
        index += 1;
        let noop_program = &accounts[index];
        index += 1;
        anchor_lang::solana_program::msg!("noop_program");
        let account_compression_authority = &accounts[index];
        index += 1;
        let account_compression_program = &accounts[index];
        index += 1;
        let self_program = &accounts[index];
        index += 1;
        let system_program = &accounts[index];
        index += 1;
        anchor_lang::solana_program::msg!("pre sol_pool_pda");
        let sol_pool_pda = if with_lamports {
            Some(&accounts[index])
        } else {
            None
        };
        if with_lamports {
            index += 1;
        }
        anchor_lang::solana_program::msg!("prost sol_pool_pda");
        let mint_in_merkle_tree = &accounts[index];
        anchor_lang::solana_program::msg!("prost sol_pool_pda");
        let mint_in_queue = &accounts[index + 1];
        anchor_lang::solana_program::msg!("prost sol_pool_pda");
        let mint_out_queue = &accounts[index + 2];
        anchor_lang::solana_program::msg!("prost sol_pool_pda");
        let tokens_out_queue = &accounts[index + 3];

        // Validate fee_payer: must be signer and mutable
        check_signer(fee_payer).map_err(ProgramError::from)?;
        check_mut(fee_payer).map_err(ProgramError::from)?;

        // Validate authority: must be signer
        check_signer(authority).map_err(ProgramError::from)?;

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
            system_program,
            sol_pool_pda,
            self_program,
            mint_in_merkle_tree,
            mint_in_queue,
            mint_out_queue,
            tokens_out_queue,
        })
    }
}
