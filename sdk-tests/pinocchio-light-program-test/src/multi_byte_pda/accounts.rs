use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::{CreateAccountsProof, LightAccount, LightDiscriminator};
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    sysvars::Sysvar,
};

use crate::state::{
    FiveByteRecord, FourByteRecord, SevenByteRecord, SixByteRecord, ThreeByteRecord, TwoByteRecord,
};

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct CreateMultiByteRecordsParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: [u8; 32],
}

pub struct CreateMultiByteRecords<'a> {
    pub fee_payer: &'a AccountInfo,
    pub compression_config: &'a AccountInfo,
    pub pda_rent_sponsor: &'a AccountInfo,
    pub two_byte_record: &'a AccountInfo,
    pub three_byte_record: &'a AccountInfo,
    pub four_byte_record: &'a AccountInfo,
    pub five_byte_record: &'a AccountInfo,
    pub six_byte_record: &'a AccountInfo,
    pub seven_byte_record: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
}

impl<'a> CreateMultiByteRecords<'a> {
    pub const FIXED_LEN: usize = 10;

    pub fn parse(
        accounts: &'a [AccountInfo],
        params: &CreateMultiByteRecordsParams,
    ) -> Result<Self, ProgramError> {
        let fee_payer = &accounts[0];
        let compression_config = &accounts[1];
        let pda_rent_sponsor = &accounts[2];
        let two_byte_record = &accounts[3];
        let three_byte_record = &accounts[4];
        let four_byte_record = &accounts[5];
        let five_byte_record = &accounts[6];
        let six_byte_record = &accounts[7];
        let seven_byte_record = &accounts[8];
        let system_program = &accounts[9];

        if !fee_payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let rent =
            pinocchio::sysvars::rent::Rent::get().map_err(|_| ProgramError::UnsupportedSysvar)?;

        // Create TwoByteRecord PDA
        {
            let disc_len = TwoByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
            let space = disc_len + TwoByteRecord::INIT_SPACE;
            let seeds: &[&[u8]] = &[b"two_byte_record", &params.owner];
            let (expected_pda, bump) = pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if two_byte_record.key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }
            let lamports = rent.minimum_balance(space);
            let bump_bytes = [bump];
            let seed_array = [
                Seed::from(b"two_byte_record" as &[u8]),
                Seed::from(params.owner.as_ref()),
                Seed::from(bump_bytes.as_ref()),
            ];
            let signer = Signer::from(&seed_array);
            pinocchio_system::instructions::CreateAccount {
                from: fee_payer,
                to: two_byte_record,
                lamports,
                space: space as u64,
                owner: &crate::ID,
            }
            .invoke_signed(&[signer])?;
            let mut data = two_byte_record
                .try_borrow_mut_data()
                .map_err(|_| ProgramError::AccountBorrowFailed)?;
            data[0..disc_len].copy_from_slice(TwoByteRecord::LIGHT_DISCRIMINATOR_SLICE);
        }

        // Create ThreeByteRecord PDA
        {
            let disc_len = ThreeByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
            let space = disc_len + ThreeByteRecord::INIT_SPACE;
            let seeds: &[&[u8]] = &[b"three_byte_record", &params.owner];
            let (expected_pda, bump) = pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if three_byte_record.key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }
            let lamports = rent.minimum_balance(space);
            let bump_bytes = [bump];
            let seed_array = [
                Seed::from(b"three_byte_record" as &[u8]),
                Seed::from(params.owner.as_ref()),
                Seed::from(bump_bytes.as_ref()),
            ];
            let signer = Signer::from(&seed_array);
            pinocchio_system::instructions::CreateAccount {
                from: fee_payer,
                to: three_byte_record,
                lamports,
                space: space as u64,
                owner: &crate::ID,
            }
            .invoke_signed(&[signer])?;
            let mut data = three_byte_record
                .try_borrow_mut_data()
                .map_err(|_| ProgramError::AccountBorrowFailed)?;
            data[0..disc_len].copy_from_slice(ThreeByteRecord::LIGHT_DISCRIMINATOR_SLICE);
        }

        // Create FourByteRecord PDA
        {
            let disc_len = FourByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
            let space = disc_len + FourByteRecord::INIT_SPACE;
            let seeds: &[&[u8]] = &[b"four_byte_record", &params.owner];
            let (expected_pda, bump) = pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if four_byte_record.key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }
            let lamports = rent.minimum_balance(space);
            let bump_bytes = [bump];
            let seed_array = [
                Seed::from(b"four_byte_record" as &[u8]),
                Seed::from(params.owner.as_ref()),
                Seed::from(bump_bytes.as_ref()),
            ];
            let signer = Signer::from(&seed_array);
            pinocchio_system::instructions::CreateAccount {
                from: fee_payer,
                to: four_byte_record,
                lamports,
                space: space as u64,
                owner: &crate::ID,
            }
            .invoke_signed(&[signer])?;
            let mut data = four_byte_record
                .try_borrow_mut_data()
                .map_err(|_| ProgramError::AccountBorrowFailed)?;
            data[0..disc_len].copy_from_slice(FourByteRecord::LIGHT_DISCRIMINATOR_SLICE);
        }

        // Create FiveByteRecord PDA
        {
            let disc_len = FiveByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
            let space = disc_len + FiveByteRecord::INIT_SPACE;
            let seeds: &[&[u8]] = &[b"five_byte_record", &params.owner];
            let (expected_pda, bump) = pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if five_byte_record.key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }
            let lamports = rent.minimum_balance(space);
            let bump_bytes = [bump];
            let seed_array = [
                Seed::from(b"five_byte_record" as &[u8]),
                Seed::from(params.owner.as_ref()),
                Seed::from(bump_bytes.as_ref()),
            ];
            let signer = Signer::from(&seed_array);
            pinocchio_system::instructions::CreateAccount {
                from: fee_payer,
                to: five_byte_record,
                lamports,
                space: space as u64,
                owner: &crate::ID,
            }
            .invoke_signed(&[signer])?;
            let mut data = five_byte_record
                .try_borrow_mut_data()
                .map_err(|_| ProgramError::AccountBorrowFailed)?;
            data[0..disc_len].copy_from_slice(FiveByteRecord::LIGHT_DISCRIMINATOR_SLICE);
        }

        // Create SixByteRecord PDA
        {
            let disc_len = SixByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
            let space = disc_len + SixByteRecord::INIT_SPACE;
            let seeds: &[&[u8]] = &[b"six_byte_record", &params.owner];
            let (expected_pda, bump) = pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if six_byte_record.key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }
            let lamports = rent.minimum_balance(space);
            let bump_bytes = [bump];
            let seed_array = [
                Seed::from(b"six_byte_record" as &[u8]),
                Seed::from(params.owner.as_ref()),
                Seed::from(bump_bytes.as_ref()),
            ];
            let signer = Signer::from(&seed_array);
            pinocchio_system::instructions::CreateAccount {
                from: fee_payer,
                to: six_byte_record,
                lamports,
                space: space as u64,
                owner: &crate::ID,
            }
            .invoke_signed(&[signer])?;
            let mut data = six_byte_record
                .try_borrow_mut_data()
                .map_err(|_| ProgramError::AccountBorrowFailed)?;
            data[0..disc_len].copy_from_slice(SixByteRecord::LIGHT_DISCRIMINATOR_SLICE);
        }

        // Create SevenByteRecord PDA
        {
            let disc_len = SevenByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
            let space = disc_len + SevenByteRecord::INIT_SPACE;
            let seeds: &[&[u8]] = &[b"seven_byte_record", &params.owner];
            let (expected_pda, bump) = pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if seven_byte_record.key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }
            let lamports = rent.minimum_balance(space);
            let bump_bytes = [bump];
            let seed_array = [
                Seed::from(b"seven_byte_record" as &[u8]),
                Seed::from(params.owner.as_ref()),
                Seed::from(bump_bytes.as_ref()),
            ];
            let signer = Signer::from(&seed_array);
            pinocchio_system::instructions::CreateAccount {
                from: fee_payer,
                to: seven_byte_record,
                lamports,
                space: space as u64,
                owner: &crate::ID,
            }
            .invoke_signed(&[signer])?;
            let mut data = seven_byte_record
                .try_borrow_mut_data()
                .map_err(|_| ProgramError::AccountBorrowFailed)?;
            data[0..disc_len].copy_from_slice(SevenByteRecord::LIGHT_DISCRIMINATOR_SLICE);
        }

        Ok(Self {
            fee_payer,
            compression_config,
            pda_rent_sponsor,
            two_byte_record,
            three_byte_record,
            four_byte_record,
            five_byte_record,
            six_byte_record,
            seven_byte_record,
            system_program,
        })
    }
}
