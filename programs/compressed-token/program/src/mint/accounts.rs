use crate::constants::BUMP_CPI_AUTHORITY;
use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::checks::{
    check_mut, check_non_mut, check_pda_seeds_with_bump, check_program, check_signer,
};
use light_compressed_account::constants::ACCOUNT_COMPRESSION_PROGRAM_ID;
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};

pub struct CreateCompressedMintAccounts<'info> {
    pub address_merkle_tree: &'info AccountInfo,
    pub mint_signer: &'info AccountInfo,
}

impl<'info> CreateCompressedMintAccounts<'info> {
    pub fn validate_and_parse(
        accounts: &'info [AccountInfo],
        program_id: &Pubkey,
    ) -> Result<Self, ProgramError> {
        if accounts.len() != 12 {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        // Static non-CPI accounts first
        let mint_signer = &accounts[0];
        let light_system_program = &accounts[1];

        // CPI accounts in exact order expected by InvokeCpiWithReadOnly
        let fee_payer = &accounts[2];
        let cpi_authority_pda = &accounts[3];
        let registered_program_pda = &accounts[4];
        let noop_program = &accounts[5];
        let account_compression_authority = &accounts[6];
        let account_compression_program = &accounts[7];
        let self_program = &accounts[8];
        // let sol_pool_pda_placeholder = &accounts[9]; // light_system_program placeholder
        // let _decompression_recipient_placeholder = &accounts[10]; // light_system_program placeholder
        let system_program = &accounts[9];
        // let _cpi_context_placeholder = &accounts[12]; // light_system_program placeholder
        let address_merkle_tree = &accounts[10];
        let output_queue = &accounts[11];

        // Validate fee_payer: must be signer and mutable
        check_signer(fee_payer).map_err(ProgramError::from)?;
        check_mut(fee_payer).map_err(ProgramError::from)?;

        // Validate cpi_authority_pda: must be the correct PDA
        let expected_seeds = &[CPI_AUTHORITY_PDA_SEED, &[BUMP_CPI_AUTHORITY]];
        check_pda_seeds_with_bump(expected_seeds, program_id, cpi_authority_pda)
            .map_err(ProgramError::from)?;

        // Validate light_system_program: must be the correct program
        // The placeholders are always None -> no need for an extra light system program account info.
        let light_system_program_id = light_system_program::id();
        check_program(&light_system_program_id.to_bytes(), light_system_program)
            .map_err(ProgramError::from)?;

        // Validate account_compression_program: must be the correct program
        check_program(&ACCOUNT_COMPRESSION_PROGRAM_ID, account_compression_program)
            .map_err(ProgramError::from)?;

        // Validate registered_program_pda: non-mutable
        check_non_mut(registered_program_pda).map_err(ProgramError::from)?;

        // Validate noop_program: non-mutable
        check_non_mut(noop_program).map_err(ProgramError::from)?;

        // Validate account_compression_authority: non-mutable
        check_non_mut(account_compression_authority).map_err(ProgramError::from)?;

        // Validate self_program: must be this program
        check_program(program_id, self_program).map_err(ProgramError::from)?;

        // Validate system_program: must be the system program
        let system_program_id = anchor_lang::solana_program::system_program::ID;
        check_program(&system_program_id.to_bytes(), system_program).map_err(ProgramError::from)?;

        // Validate address_merkle_tree: mutable
        check_mut(address_merkle_tree).map_err(ProgramError::from)?;

        // Validate output_queue: mutable
        check_mut(output_queue).map_err(ProgramError::from)?;

        // Validate mint_signer: must be signer
        check_signer(mint_signer).map_err(ProgramError::from)?;

        Ok(CreateCompressedMintAccounts {
            address_merkle_tree,
            mint_signer,
        })
    }
}
