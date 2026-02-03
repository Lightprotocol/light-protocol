//! Accounts struct for create_derived_mints instruction (pinocchio version).

use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::CreateAccountsProof;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

/// Seed constants
pub const MINT_SIGNER_0_SEED: &[u8] = b"mint_signer_0";
pub const MINT_SIGNER_1_SEED: &[u8] = b"mint_signer_1";

/// Minimal params - matches macro pattern.
#[derive(Clone, BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateDerivedMintsParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub mint_signer_0_bump: u8,
    pub mint_signer_1_bump: u8,
}

/// Accounts struct - matches macro pattern with mint signers as PDAs.
pub struct CreateDerivedMintsAccounts<'a> {
    pub payer: &'a AccountInfo,
    pub authority: &'a AccountInfo,
    pub mint_signer_0: &'a AccountInfo,
    pub mint_signer_1: &'a AccountInfo,
    pub mint_0: &'a AccountInfo,
    pub mint_1: &'a AccountInfo,
    pub compressible_config: &'a AccountInfo,
    pub rent_sponsor: &'a AccountInfo,
    pub light_token_program: &'a AccountInfo,
    pub cpi_authority: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    /// Slice view for mint_signer accounts (for invoke_create_mints)
    pub mint_signers_slice: &'a [AccountInfo],
    /// Slice view for mint accounts (for invoke_create_mints)
    pub mints_slice: &'a [AccountInfo],
}

impl<'a> CreateDerivedMintsAccounts<'a> {
    pub const FIXED_LEN: usize = 11;

    pub fn parse(
        accounts: &'a [AccountInfo],
        params: &CreateDerivedMintsParams,
    ) -> Result<Self, ProgramError> {
        let payer = &accounts[0];
        let authority = &accounts[1];
        let mint_signer_0 = &accounts[2];
        let mint_signer_1 = &accounts[3];
        let mint_0 = &accounts[4];
        let mint_1 = &accounts[5];
        let compressible_config = &accounts[6];
        let rent_sponsor = &accounts[7];
        let light_token_program = &accounts[8];
        let cpi_authority = &accounts[9];
        let system_program = &accounts[10];

        // Validate signers
        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !authority.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate mint_signer_0 PDA
        {
            let authority_key = authority.key();
            let seeds: &[&[u8]] = &[MINT_SIGNER_0_SEED, authority_key];
            let (expected_pda, expected_bump) =
                pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if mint_signer_0.key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }
            if expected_bump != params.mint_signer_0_bump {
                return Err(ProgramError::InvalidSeeds);
            }
        }

        // Validate mint_signer_1 PDA
        {
            let authority_key = authority.key();
            let seeds: &[&[u8]] = &[MINT_SIGNER_1_SEED, authority_key];
            let (expected_pda, expected_bump) =
                pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if mint_signer_1.key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }
            if expected_bump != params.mint_signer_1_bump {
                return Err(ProgramError::InvalidSeeds);
            }
        }

        Ok(Self {
            payer,
            authority,
            mint_signer_0,
            mint_signer_1,
            mint_0,
            mint_1,
            compressible_config,
            rent_sponsor,
            light_token_program,
            cpi_authority,
            system_program,
            mint_signers_slice: &accounts[2..4],
            mints_slice: &accounts[4..6],
        })
    }
}
