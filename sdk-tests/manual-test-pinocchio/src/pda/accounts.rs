//! Accounts module for single-pda-test (pinocchio version).

use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::CreateAccountsProof;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    sysvars::Sysvar,
};

use super::state::MinimalRecord;

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct CreatePdaParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: [u8; 32],
    pub nonce: u64,
}

/// Minimal accounts struct for testing single PDA creation.
pub struct CreatePda<'a> {
    pub fee_payer: &'a AccountInfo,
    pub compression_config: &'a AccountInfo,
    pub record: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
}

impl<'a> CreatePda<'a> {
    pub const FIXED_LEN: usize = 4;

    pub fn parse(
        accounts: &'a [AccountInfo],
        params: &CreatePdaParams,
    ) -> Result<Self, ProgramError> {
        let fee_payer = &accounts[0];
        let compression_config = &accounts[1];
        let record = &accounts[2];
        let system_program = &accounts[3];

        if !fee_payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Verify and create the PDA account via system program CPI
        let space = 8 + MinimalRecord::INIT_SPACE;
        let nonce_bytes = params.nonce.to_le_bytes();
        let seeds: &[&[u8]] = &[b"minimal_record", &params.owner, &nonce_bytes];
        let (expected_pda, bump) = pinocchio::pubkey::find_program_address(seeds, &crate::ID);
        if record.key() != &expected_pda {
            return Err(ProgramError::InvalidSeeds);
        }

        let rent = pinocchio::sysvars::rent::Rent::get()
            .map_err(|_| ProgramError::UnsupportedSysvar)?;
        let lamports = rent.minimum_balance(space);

        let bump_bytes = [bump];
        let seed_array = [
            Seed::from(b"minimal_record" as &[u8]),
            Seed::from(params.owner.as_ref()),
            Seed::from(nonce_bytes.as_ref()),
            Seed::from(bump_bytes.as_ref()),
        ];
        let signer = Signer::from(&seed_array);
        pinocchio_system::instructions::CreateAccount {
            from: fee_payer,
            to: record,
            lamports,
            space: space as u64,
            owner: &crate::ID,
        }
        .invoke_signed(&[signer])?;

        // Write LIGHT_DISCRIMINATOR to first 8 bytes
        {
            use light_account_pinocchio::LightDiscriminator;
            let mut data = record
                .try_borrow_mut_data()
                .map_err(|_| ProgramError::AccountBorrowFailed)?;
            data[..8].copy_from_slice(&MinimalRecord::LIGHT_DISCRIMINATOR);
        }

        Ok(Self {
            fee_payer,
            compression_config,
            record,
            system_program,
        })
    }
}
