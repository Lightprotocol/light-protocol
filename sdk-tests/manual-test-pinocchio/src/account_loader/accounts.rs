//! Accounts module for zero-copy account instruction (pinocchio version).

use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::CreateAccountsProof;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    sysvars::Sysvar,
};

use super::state::ZeroCopyRecord;

/// Parameters for creating a zero-copy compressible PDA.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct CreateZeroCopyParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: [u8; 32],
    pub value: u64,
    pub name: String,
}

/// Accounts struct for creating a zero-copy compressible PDA.
pub struct CreateZeroCopy<'a> {
    pub fee_payer: &'a AccountInfo,
    pub compression_config: &'a AccountInfo,
    pub record: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
}

impl<'a> CreateZeroCopy<'a> {
    pub const FIXED_LEN: usize = 4;

    pub fn parse(
        accounts: &'a [AccountInfo],
        params: &CreateZeroCopyParams,
    ) -> Result<Self, ProgramError> {
        let fee_payer = &accounts[0];
        let compression_config = &accounts[1];
        let record = &accounts[2];
        let system_program = &accounts[3];

        if !fee_payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Verify and create the PDA account via system program CPI
        let space = 8 + ZeroCopyRecord::INIT_SPACE;
        let name_bytes = params.name.as_bytes();
        let seeds: &[&[u8]] = &[b"zero_copy", &params.owner, name_bytes];
        let (expected_pda, bump) = pinocchio::pubkey::find_program_address(seeds, &crate::ID);
        if record.key() != &expected_pda {
            return Err(ProgramError::InvalidSeeds);
        }

        let rent = pinocchio::sysvars::rent::Rent::get()
            .map_err(|_| ProgramError::UnsupportedSysvar)?;
        let lamports = rent.minimum_balance(space);

        let bump_bytes = [bump];
        let seed_array = [
            Seed::from(b"zero_copy" as &[u8]),
            Seed::from(params.owner.as_ref()),
            Seed::from(name_bytes),
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
            data[..8].copy_from_slice(&ZeroCopyRecord::LIGHT_DISCRIMINATOR);
        }

        Ok(Self {
            fee_payer,
            compression_config,
            record,
            system_program,
        })
    }
}
