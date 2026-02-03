use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::CreateAccountsProof;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateMintParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub mint_signer_bump: u8,
}

pub struct CreateMintAccounts<'a> {
    pub payer: &'a AccountInfo,
    pub authority: &'a AccountInfo,
    pub mint_signer: &'a AccountInfo,
    pub mint: &'a AccountInfo,
    pub compressible_config: &'a AccountInfo,
    pub rent_sponsor: &'a AccountInfo,
    pub light_token_program: &'a AccountInfo,
    pub cpi_authority: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
}

impl<'a> CreateMintAccounts<'a> {
    pub const FIXED_LEN: usize = 9;

    pub fn parse(
        accounts: &'a [AccountInfo],
        params: &CreateMintParams,
    ) -> Result<Self, ProgramError> {
        let payer = &accounts[0];
        let authority = &accounts[1];
        let mint_signer = &accounts[2];
        let mint = &accounts[3];
        let compressible_config = &accounts[4];
        let rent_sponsor = &accounts[5];
        let light_token_program = &accounts[6];
        let cpi_authority = &accounts[7];
        let system_program = &accounts[8];

        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !authority.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate mint_signer PDA
        {
            let authority_key = authority.key();
            let seeds: &[&[u8]] = &[crate::MINT_SIGNER_SEED_A, authority_key];
            let (expected_pda, expected_bump) =
                pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if mint_signer.key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }
            if expected_bump != params.mint_signer_bump {
                return Err(ProgramError::InvalidSeeds);
            }
        }

        Ok(Self {
            payer,
            authority,
            mint_signer,
            mint,
            compressible_config,
            rent_sponsor,
            light_token_program,
            cpi_authority,
            system_program,
        })
    }
}
