use anchor_lang::solana_program::program_error::ProgramError;
use light_program_profiler::profile;
use pinocchio::{
    address::Address,
    cpi::{Seed, Signer},
    sysvars::{rent::Rent, Sysvar},
    AccountView as AccountInfo,
};
use pinocchio_system::instructions::{Assign, CreateAccount, Transfer};

use crate::{shared::convert_program_error, LIGHT_CPI_SIGNER};

/// Creates an account with explicit seed parameters for fee_payer and new_account.
///
/// ## Parameters
/// - `fee_payer`: Account paying for rent (keypair or PDA like rent_sponsor)
/// - `new_account`: Account being created (keypair or PDA like ATA)
/// - `account_size`: Size in bytes for the new account
/// - `fee_payer_seeds`: PDA seeds if fee_payer is a PDA (e.g., rent_sponsor), None if keypair
/// - `new_account_seeds`: PDA seeds if new_account is a PDA (e.g., ATA), None if keypair
/// - `additional_lamports`: Extra lamports beyond rent-exempt minimum (e.g., compression cost)
///
/// ## Cold Path
/// If new_account already has lamports (e.g., attacker donation), uses
/// Assign + realloc + Transfer pattern instead of CreateAccount which would fail.
#[profile]
pub fn create_pda_account(
    fee_payer: &AccountInfo,
    new_account: &AccountInfo,
    account_size: usize,
    fee_payer_seeds: Option<&[Seed]>,
    new_account_seeds: Option<&[Seed]>,
    additional_lamports: Option<u64>,
) -> Result<(), ProgramError> {
    // Calculate rent
    let rent = Rent::get().map_err(|_| ProgramError::UnsupportedSysvar)?;
    let lamports = rent
        .try_minimum_balance(account_size)
        .map_err(|_| ProgramError::ArithmeticOverflow)?
        + additional_lamports.unwrap_or_default();

    // Build signers from seeds
    let fee_payer_signer: Option<Signer> = fee_payer_seeds.map(Signer::from);
    let new_account_signer: Option<Signer> = new_account_seeds.map(Signer::from);

    // Cold Path: if account already has lamports (e.g., from attacker donation),
    // use Assign + realloc + Transfer instead of CreateAccount which would fail.
    if new_account.lamports() > 0 {
        // Verify account is owned by system program (uninitialized).
        // Prevents overwriting an already-initialized account.
        if !new_account.owned_by(&Address::from([0u8; 32])) {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        let current_lamports = new_account.lamports();

        let owner_addr = Address::from(LIGHT_CPI_SIGNER.program_id);
        Assign {
            account: new_account,
            owner: &owner_addr,
        }
        .invoke_signed(new_account_signer.as_slice())
        .map_err(convert_program_error)?;

        new_account
            .resize(account_size)
            .map_err(convert_program_error)?;

        // Transfer remaining lamports for rent-exemption if needed
        if lamports > current_lamports {
            Transfer {
                from: fee_payer,
                to: new_account,
                lamports: lamports - current_lamports,
            }
            .invoke_signed(fee_payer_signer.as_slice())
            .map_err(convert_program_error)?;
        }

        return Ok(());
    }

    // Normal path: CreateAccount (requires both to sign)
    let mut signers = arrayvec::ArrayVec::<Signer, 2>::new();
    if let Some(s) = fee_payer_signer {
        signers.push(s);
    }
    if let Some(s) = new_account_signer {
        signers.push(s);
    }

    let owner_addr = Address::from(LIGHT_CPI_SIGNER.program_id);
    CreateAccount {
        from: fee_payer,
        to: new_account,
        lamports,
        space: account_size as u64,
        owner: &owner_addr,
    }
    .invoke_signed(signers.as_slice())
    .map_err(convert_program_error)
}

/// Verifies that the provided account matches the expected PDA.
/// Derives the canonical bump on-chain using `find_program_address`.
/// Returns the canonical bump on success.
pub fn verify_pda<const N: usize>(
    account_key: &[u8; 32],
    seeds: &[&[u8]; N],
    program_id: &[u8; 32],
) -> Result<u8, ProgramError> {
    let program_addr = Address::from(*program_id);
    let (expected_pubkey, bump) = Address::find_program_address(seeds.as_slice(), &program_addr);

    if *account_key != expected_pubkey.to_bytes() {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(bump)
}
