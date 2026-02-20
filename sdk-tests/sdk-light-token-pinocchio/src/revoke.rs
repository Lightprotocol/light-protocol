use light_token_pinocchio::instruction::RevokeCpi;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
};

use crate::{ID, TOKEN_ACCOUNT_SEED};

/// Handler for revoking delegation on a Light Token account (invoke)
///
/// Account order:
/// - accounts[0]: token_account (writable)
/// - accounts[1]: owner (signer)
/// - accounts[2]: system_program
/// - accounts[3]: light_token_program
pub fn process_revoke_invoke(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    if accounts.len() < 4 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    RevokeCpi {
        token_account: &accounts[0],
        owner: &accounts[1],
        system_program: &accounts[2],
        fee_payer: &accounts[1],
    }
    .invoke()?;

    Ok(())
}

/// Handler for revoking delegation on a PDA-owned Light Token account (invoke_signed)
///
/// Account order:
/// - accounts[0]: token_account (writable)
/// - accounts[1]: PDA owner (program signs)
/// - accounts[2]: system_program
/// - accounts[3]: light_token_program
/// - accounts[4]: fee_payer (writable, signer)
pub fn process_revoke_invoke_signed(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    if accounts.len() < 5 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the owner
    let (pda, bump) = pinocchio::pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &ID);

    // Verify the owner account is the PDA we expect
    if pda != *accounts[1].key() {
        return Err(ProgramError::InvalidSeeds);
    }

    let bump_byte = [bump];
    let seeds = [Seed::from(TOKEN_ACCOUNT_SEED), Seed::from(&bump_byte[..])];
    let signer = Signer::from(&seeds);

    RevokeCpi {
        token_account: &accounts[0],
        owner: &accounts[1],
        system_program: &accounts[2],
        fee_payer: &accounts[4],
    }
    .invoke_signed(&[signer])?;

    Ok(())
}

/// Handler for revoking delegation with a separate fee_payer (invoke)
///
/// Account order:
/// - accounts[0]: token_account (writable)
/// - accounts[1]: owner (signer)
/// - accounts[2]: system_program
/// - accounts[3]: light_token_program
/// - accounts[4]: fee_payer (writable, signer)
pub fn process_revoke_invoke_with_fee_payer(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    if accounts.len() < 5 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    RevokeCpi {
        token_account: &accounts[0],
        owner: &accounts[1],
        system_program: &accounts[2],
        fee_payer: &accounts[4],
    }
    .invoke()?;

    Ok(())
}
