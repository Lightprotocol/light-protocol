//! Accounts struct for create_token_vault instruction (pinocchio version).

use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

/// Seed constant for token vault PDA
pub const TOKEN_VAULT_SEED: &[u8] = b"vault";

/// Minimal params for token vault creation.
#[derive(Clone, BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateTokenVaultParams {
    pub vault_bump: u8,
}

/// Accounts struct for creating a PDA token vault.
///
/// The token vault is created via CPI to light-token program, not system program.
/// parse() only validates the PDA derivation.
pub struct CreateTokenVaultAccounts<'a> {
    pub payer: &'a AccountInfo,
    pub mint: &'a AccountInfo,
    pub vault_owner: &'a AccountInfo,
    pub token_vault: &'a AccountInfo,
    pub compressible_config: &'a AccountInfo,
    pub rent_sponsor: &'a AccountInfo,
    pub light_token_program: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
}

impl<'a> CreateTokenVaultAccounts<'a> {
    pub const FIXED_LEN: usize = 8;

    pub fn parse(accounts: &'a [AccountInfo]) -> Result<Self, ProgramError> {
        let payer = &accounts[0];
        let mint = &accounts[1];
        let vault_owner = &accounts[2];
        let token_vault = &accounts[3];
        let compressible_config = &accounts[4];
        let rent_sponsor = &accounts[5];
        let light_token_program = &accounts[6];
        let system_program = &accounts[7];

        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate token_vault PDA
        {
            let mint_key = mint.key();
            let seeds: &[&[u8]] = &[TOKEN_VAULT_SEED, mint_key];
            let (expected_pda, _bump) = pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if token_vault.key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }
        }

        Ok(Self {
            payer,
            mint,
            vault_owner,
            token_vault,
            compressible_config,
            rent_sponsor,
            light_token_program,
            system_program,
        })
    }
}
