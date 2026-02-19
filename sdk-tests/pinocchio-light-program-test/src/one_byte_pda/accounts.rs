use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::{CreateAccountsProof, LightAccount, LightDiscriminator};
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    sysvars::Sysvar,
};

use crate::state::OneByteRecord;

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct CreateOneByteRecordParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: [u8; 32],
}

pub struct CreateOneByteRecord<'a> {
    pub fee_payer: &'a AccountInfo,
    pub compression_config: &'a AccountInfo,
    pub pda_rent_sponsor: &'a AccountInfo,
    pub record: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
}

impl<'a> CreateOneByteRecord<'a> {
    pub const FIXED_LEN: usize = 5;

    pub fn parse(
        accounts: &'a [AccountInfo],
        params: &CreateOneByteRecordParams,
    ) -> Result<Self, ProgramError> {
        let fee_payer = &accounts[0];
        let compression_config = &accounts[1];
        let pda_rent_sponsor = &accounts[2];
        let record = &accounts[3];
        let system_program = &accounts[4];

        if !fee_payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Derive PDA with discriminator layout: space = disc_len + INIT_SPACE
        let disc_len = OneByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
        let space = disc_len + OneByteRecord::INIT_SPACE;
        let seeds: &[&[u8]] = &[b"one_byte_record", &params.owner];
        let (expected_pda, bump) = pinocchio::pubkey::find_program_address(seeds, &crate::ID);
        if record.key() != &expected_pda {
            return Err(ProgramError::InvalidSeeds);
        }

        let rent =
            pinocchio::sysvars::rent::Rent::get().map_err(|_| ProgramError::UnsupportedSysvar)?;
        let lamports = rent.minimum_balance(space);

        let bump_bytes = [bump];
        let seed_array = [
            Seed::from(b"one_byte_record" as &[u8]),
            Seed::from(params.owner.as_ref()),
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

        // Write discriminator to data[0..disc_len]
        {
            let mut data = record
                .try_borrow_mut_data()
                .map_err(|_| ProgramError::AccountBorrowFailed)?;
            data[0..disc_len].copy_from_slice(OneByteRecord::LIGHT_DISCRIMINATOR_SLICE);
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
