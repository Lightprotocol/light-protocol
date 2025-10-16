use anchor_lang::solana_program::program_error::ProgramError;
use light_program_profiler::profile;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    pubkey::Pubkey,
    sysvars::{rent::Rent, Sysvar},
};
use pinocchio_system::instructions::CreateAccount;

use crate::{shared::convert_program_error, LIGHT_CPI_SIGNER};

// /// Configuration for creating a PDA account
// #[derive(Debug)]
// pub struct CreatePdaSeeds<'a> {
//     /// The seeds used to derive the PDA (without bump)
//     pub seeds: &'a [&'a [u8]],
//     /// The bump seed for PDA derivation
//     pub bump: u8,
// }

/// Creates a PDA account with the specified configuration(s).
///
/// This function abstracts the common PDA account creation pattern used across
/// create_associated_token_account, create_mint_account, and create_token_pool.
///
/// ## Process
/// 1. Calculates rent based on account size
/// 2. Builds seed arrays with bumps for each config
/// 3. Creates account via system program with specified owner
/// 4. Signs transaction with derived PDA seeds
///
/// ## Parameters
/// - `configs`: ArrayVec of PDA configs. First config is for the new account being created.
///              Additional configs are for fee payer PDAs that need to sign.
#[profile]
pub fn create_pda_account<const N: usize>(
    fee_payer: &AccountInfo,
    new_account: &AccountInfo,
    account_size: usize,
    seeds_inputs: [&[Seed]; N],
    additional_lamports: Option<u64>,
) -> Result<(), ProgramError> {
    // Ensure we have at least one config
    if seeds_inputs.is_empty() {
        return Err(ProgramError::InvalidInstructionData);
    }
    // Calculate rent
    let rent = Rent::get().map_err(|_| ProgramError::UnsupportedSysvar)?;
    let lamports = rent.minimum_balance(account_size) + additional_lamports.unwrap_or_default();

    let create_account = CreateAccount {
        from: fee_payer,
        to: new_account,
        lamports,
        space: account_size as u64,
        owner: &LIGHT_CPI_SIGNER.program_id,
    };

    let mut signers = arrayvec::ArrayVec::<Signer, N>::new();
    for seeds in seeds_inputs.iter() {
        if !seeds.is_empty() {
            signers.push(Signer::from(*seeds));
        }
    }

    create_account
        .invoke_signed(signers.as_slice())
        .map_err(convert_program_error)
}

/// Verifies that the provided account matches the expected PDA
pub fn verify_pda<const N: usize>(
    account_key: &[u8; 32],
    seeds: &[&[u8]; N],
    bump: u8,
    program_id: &Pubkey,
) -> Result<(), ProgramError> {
    let expected_pubkey = pinocchio_pubkey::derive_address(seeds, Some(bump), program_id);

    if account_key != &expected_pubkey {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}
