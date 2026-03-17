use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::{CreateAccountsProof, LightAccount};
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    sysvars::Sysvar,
};

use crate::state::{MinimalRecord, OneByteRecord, ZeroCopyRecord};

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateAllParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: [u8; 32],
    pub mint_signer_bump: u8,
    pub token_vault_bump: u8,
}

pub struct CreateAllAccounts<'a> {
    pub payer: &'a AccountInfo,
    pub authority: &'a AccountInfo,
    pub compression_config: &'a AccountInfo,
    pub borsh_record: &'a AccountInfo,
    pub zero_copy_record: &'a AccountInfo,
    pub one_byte_record: &'a AccountInfo,
    pub mint_signer: &'a AccountInfo,
    pub mint: &'a AccountInfo,
    pub token_vault: &'a AccountInfo,
    pub vault_owner: &'a AccountInfo,
    pub ata_owner: &'a AccountInfo,
    pub user_ata: &'a AccountInfo,
    pub compressible_config: &'a AccountInfo,
    pub rent_sponsor: &'a AccountInfo,
    pub light_token_program: &'a AccountInfo,
    pub cpi_authority: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub mint_signers_slice: &'a [AccountInfo],
    pub mints_slice: &'a [AccountInfo],
}

impl<'a> CreateAllAccounts<'a> {
    pub const FIXED_LEN: usize = 17;

    pub fn parse(
        accounts: &'a [AccountInfo],
        params: &CreateAllParams,
    ) -> Result<Self, ProgramError> {
        let payer = &accounts[0];
        let authority = &accounts[1];
        let compression_config = &accounts[2];
        let borsh_record = &accounts[3];
        let zero_copy_record = &accounts[4];
        let one_byte_record = &accounts[5];
        let mint_signer = &accounts[6];
        let mint = &accounts[7];
        let token_vault = &accounts[8];
        let vault_owner = &accounts[9];
        let ata_owner = &accounts[10];
        let user_ata = &accounts[11];
        let compressible_config = &accounts[12];
        let rent_sponsor = &accounts[13];
        let light_token_program = &accounts[14];
        let cpi_authority = &accounts[15];
        let system_program = &accounts[16];

        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !authority.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Create Borsh PDA
        {
            let space = 8 + MinimalRecord::INIT_SPACE;
            let seeds: &[&[u8]] = &[b"minimal_record", &params.owner];
            let (expected_pda, bump) = pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if borsh_record.key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }

            let rent = pinocchio::sysvars::rent::Rent::get()
                .map_err(|_| ProgramError::UnsupportedSysvar)?;
            let lamports = rent.minimum_balance(space);

            let bump_bytes = [bump];
            let seed_array = [
                Seed::from(b"minimal_record" as &[u8]),
                Seed::from(params.owner.as_ref()),
                Seed::from(bump_bytes.as_ref()),
            ];
            let signer = Signer::from(&seed_array);
            pinocchio_system::instructions::CreateAccount {
                from: payer,
                to: borsh_record,
                lamports,
                space: space as u64,
                owner: &crate::ID,
            }
            .invoke_signed(&[signer])?;

            use light_account_pinocchio::LightDiscriminator;
            let mut data = borsh_record
                .try_borrow_mut_data()
                .map_err(|_| ProgramError::AccountBorrowFailed)?;
            data[..8].copy_from_slice(&MinimalRecord::LIGHT_DISCRIMINATOR);
        }

        // Create ZeroCopy PDA
        {
            let space = 8 + ZeroCopyRecord::INIT_SPACE;
            let seeds: &[&[u8]] = &[crate::RECORD_SEED, &params.owner];
            let (expected_pda, bump) = pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if zero_copy_record.key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }

            let rent = pinocchio::sysvars::rent::Rent::get()
                .map_err(|_| ProgramError::UnsupportedSysvar)?;
            let lamports = rent.minimum_balance(space);

            let bump_bytes = [bump];
            let seed_array = [
                Seed::from(crate::RECORD_SEED),
                Seed::from(params.owner.as_ref()),
                Seed::from(bump_bytes.as_ref()),
            ];
            let signer = Signer::from(&seed_array);
            pinocchio_system::instructions::CreateAccount {
                from: payer,
                to: zero_copy_record,
                lamports,
                space: space as u64,
                owner: &crate::ID,
            }
            .invoke_signed(&[signer])?;

            use light_account_pinocchio::LightDiscriminator;
            let mut data = zero_copy_record
                .try_borrow_mut_data()
                .map_err(|_| ProgramError::AccountBorrowFailed)?;
            data[..8].copy_from_slice(&ZeroCopyRecord::LIGHT_DISCRIMINATOR);
        }

        // Create OneByteRecord PDA
        {
            use light_account_pinocchio::LightDiscriminator;
            let disc_len = OneByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
            let space = disc_len + OneByteRecord::INIT_SPACE;
            let seeds: &[&[u8]] = &[b"one_byte_record", &params.owner];
            let (expected_pda, bump) = pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if one_byte_record.key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }
            let rent = pinocchio::sysvars::rent::Rent::get()
                .map_err(|_| ProgramError::UnsupportedSysvar)?;
            let lamports = rent.minimum_balance(space);
            let bump_bytes = [bump];
            let seed_array = [
                Seed::from(b"one_byte_record" as &[u8]),
                Seed::from(params.owner.as_ref()),
                Seed::from(bump_bytes.as_ref()),
            ];
            let signer = Signer::from(&seed_array);
            pinocchio_system::instructions::CreateAccount {
                from: payer,
                to: one_byte_record,
                lamports,
                space: space as u64,
                owner: &crate::ID,
            }
            .invoke_signed(&[signer])?;

            let mut data = one_byte_record
                .try_borrow_mut_data()
                .map_err(|_| ProgramError::AccountBorrowFailed)?;
            data[..disc_len].copy_from_slice(OneByteRecord::LIGHT_DISCRIMINATOR_SLICE);
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

        // Validate token_vault PDA
        {
            let mint_key = mint.key();
            let seeds: &[&[u8]] = &[crate::VAULT_SEED, mint_key];
            let (expected_pda, expected_bump) =
                pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if token_vault.key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }
            if expected_bump != params.token_vault_bump {
                return Err(ProgramError::InvalidSeeds);
            }
        }

        Ok(Self {
            payer,
            authority,
            compression_config,
            borsh_record,
            zero_copy_record,
            one_byte_record,
            mint_signer,
            mint,
            token_vault,
            vault_owner,
            ata_owner,
            user_ata,
            compressible_config,
            rent_sponsor,
            light_token_program,
            cpi_authority,
            system_program,
            mint_signers_slice: &accounts[6..7],
            mints_slice: &accounts[7..8],
        })
    }
}
