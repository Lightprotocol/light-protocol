use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::CreateAccountsProof;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateTwoMintsParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub mint_signer_bump_a: u8,
    pub mint_signer_bump_b: u8,
}

pub struct CreateTwoMintsAccounts<'a> {
    pub payer: &'a AccountInfo,
    pub authority: &'a AccountInfo,
    pub mint_signer_a: &'a AccountInfo,
    pub mint_signer_b: &'a AccountInfo,
    pub mint_a: &'a AccountInfo,
    pub mint_b: &'a AccountInfo,
    pub compressible_config: &'a AccountInfo,
    pub rent_sponsor: &'a AccountInfo,
    pub light_token_program: &'a AccountInfo,
    pub cpi_authority: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub mint_signers_slice: &'a [AccountInfo],
    pub mints_slice: &'a [AccountInfo],
}

impl<'a> CreateTwoMintsAccounts<'a> {
    pub const FIXED_LEN: usize = 11;

    pub fn parse(
        accounts: &'a [AccountInfo],
        params: &CreateTwoMintsParams,
    ) -> Result<Self, ProgramError> {
        let payer = &accounts[0];
        let authority = &accounts[1];
        let mint_signer_a = &accounts[2];
        let mint_signer_b = &accounts[3];
        let mint_a = &accounts[4];
        let mint_b = &accounts[5];
        let compressible_config = &accounts[6];
        let rent_sponsor = &accounts[7];
        let light_token_program = &accounts[8];
        let cpi_authority = &accounts[9];
        let system_program = &accounts[10];

        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !authority.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate mint_signer_a PDA
        {
            let authority_key = authority.key();
            let seeds: &[&[u8]] = &[crate::MINT_SIGNER_SEED_A, authority_key];
            let (expected_pda, expected_bump) =
                pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if mint_signer_a.key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }
            if expected_bump != params.mint_signer_bump_a {
                return Err(ProgramError::InvalidSeeds);
            }
        }

        // Validate mint_signer_b PDA
        {
            let authority_key = authority.key();
            let seeds: &[&[u8]] = &[crate::MINT_SIGNER_SEED_B, authority_key];
            let (expected_pda, expected_bump) =
                pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if mint_signer_b.key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }
            if expected_bump != params.mint_signer_bump_b {
                return Err(ProgramError::InvalidSeeds);
            }
        }

        Ok(Self {
            payer,
            authority,
            mint_signer_a,
            mint_signer_b,
            mint_a,
            mint_b,
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
