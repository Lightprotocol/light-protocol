use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::{CreateAccountsProof, LightAccount};
use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::Sysvar,
    AccountView as AccountInfo,
};

use crate::state::ZeroCopyRecord;

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct CreateZeroCopyRecordParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: [u8; 32],
}

pub struct CreateZeroCopyRecord<'a> {
    pub fee_payer: &'a AccountInfo,
    pub compression_config: &'a AccountInfo,
    pub pda_rent_sponsor: &'a AccountInfo,
    pub record: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
}

impl<'a> CreateZeroCopyRecord<'a> {
    pub const FIXED_LEN: usize = 5;

    pub fn parse(
        accounts: &'a [AccountInfo],
        params: &CreateZeroCopyRecordParams,
    ) -> Result<Self, ProgramError> {
        let fee_payer = &accounts[0];
        let compression_config = &accounts[1];
        let pda_rent_sponsor = &accounts[2];
        let record = &accounts[3];
        let system_program = &accounts[4];

        if !fee_payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let space = 8 + ZeroCopyRecord::INIT_SPACE;
        let seeds: &[&[u8]] = &[crate::RECORD_SEED, &params.owner];
        let (expected_pda, bump) =
            pinocchio::Address::find_program_address(seeds, &pinocchio::Address::from(crate::ID));
        if record.address() != &expected_pda {
            return Err(ProgramError::InvalidSeeds);
        }

        let rent =
            pinocchio::sysvars::rent::Rent::get().map_err(|_| ProgramError::UnsupportedSysvar)?;
        let lamports = rent
            .try_minimum_balance(space)
            .map_err(|_| ProgramError::ArithmeticOverflow)?;

        let bump_bytes = [bump];
        let seed_array = [
            Seed::from(crate::RECORD_SEED),
            Seed::from(params.owner.as_ref()),
            Seed::from(bump_bytes.as_ref()),
        ];
        let signer = Signer::from(&seed_array);
        pinocchio_system::instructions::CreateAccount {
            from: fee_payer,
            to: record,
            lamports,
            space: space as u64,
            owner: &pinocchio::Address::from(crate::ID),
        }
        .invoke_signed(&[signer])?;

        // Write LIGHT_DISCRIMINATOR
        {
            use light_account_pinocchio::LightDiscriminator;
            let mut data = record
                .try_borrow_mut()
                .map_err(|_| ProgramError::AccountBorrowFailed)?;
            data[..8].copy_from_slice(&ZeroCopyRecord::LIGHT_DISCRIMINATOR);
        }

        Ok(Self {
            fee_payer,
            compression_config,
            pda_rent_sponsor,
            record,
            system_program,
        })
    }
}
