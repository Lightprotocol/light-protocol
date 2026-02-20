use borsh::{BorshDeserialize, BorshSerialize};
use light_token_pinocchio::instruction::BurnCpi;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
};

use crate::{ID, TOKEN_ACCOUNT_SEED};

/// Instruction data for burn operations
#[derive(BorshSerialize, BorshDeserialize)]
pub struct BurnData {
    pub amount: u64,
}

/// Handler for burning CTokens (invoke)
///
/// Account order:
/// - accounts[0]: source (Light Token account, writable)
/// - accounts[1]: mint (writable)
/// - accounts[2]: authority (owner, signer)
/// - accounts[3]: light_token_program
/// - accounts[4]: system_program
pub fn process_burn_invoke(accounts: &[AccountInfo], amount: u64) -> Result<(), ProgramError> {
    if accounts.len() < 5 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    BurnCpi {
        source: &accounts[0],
        mint: &accounts[1],
        amount,
        authority: &accounts[2],
        system_program: &accounts[4],
        fee_payer: &accounts[2],
    }
    .invoke()?;

    Ok(())
}

/// Handler for burning CTokens with PDA authority (invoke_signed)
///
/// Account order:
/// - accounts[0]: source (Light Token account, writable)
/// - accounts[1]: mint (writable)
/// - accounts[2]: PDA authority (owner, program signs)
/// - accounts[3]: light_token_program
/// - accounts[4]: system_program
/// - accounts[5]: fee_payer (writable, signer)
pub fn process_burn_invoke_signed(
    accounts: &[AccountInfo],
    amount: u64,
) -> Result<(), ProgramError> {
    if accounts.len() < 6 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the token account owner
    let (pda, bump) = pinocchio::pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &ID);

    // Verify the authority account is the PDA we expect
    if pda != *accounts[2].key() {
        return Err(ProgramError::InvalidSeeds);
    }

    let bump_byte = [bump];
    let seeds = [Seed::from(TOKEN_ACCOUNT_SEED), Seed::from(&bump_byte[..])];
    let signer = Signer::from(&seeds);
    BurnCpi {
        source: &accounts[0],
        mint: &accounts[1],
        amount,
        authority: &accounts[2],
        system_program: &accounts[4],
        fee_payer: &accounts[5],
    }
    .invoke_signed(&[signer])?;

    Ok(())
}

/// Handler for burning CTokens with a separate fee_payer (invoke)
///
/// Account order:
/// - accounts[0]: source (Light Token account, writable)
/// - accounts[1]: mint (writable)
/// - accounts[2]: authority (owner, signer)
/// - accounts[3]: light_token_program
/// - accounts[4]: system_program
/// - accounts[5]: fee_payer (writable, signer)
pub fn process_burn_invoke_with_fee_payer(
    accounts: &[AccountInfo],
    amount: u64,
) -> Result<(), ProgramError> {
    if accounts.len() < 6 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    BurnCpi {
        source: &accounts[0],
        mint: &accounts[1],
        amount,
        authority: &accounts[2],
        system_program: &accounts[4],
        fee_payer: &accounts[5],
    }
    .invoke()?;

    Ok(())
}
