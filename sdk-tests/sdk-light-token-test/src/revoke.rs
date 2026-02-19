use light_token::instruction::RevokeCpi;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

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
        token_account: accounts[0].clone(),
        owner: accounts[1].clone(),
        system_program: accounts[2].clone(),
        max_top_up: None,
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
pub fn process_revoke_invoke_signed(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    if accounts.len() < 4 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the owner
    let (pda, bump) = Pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &ID);

    // Verify the owner account is the PDA we expect
    if &pda != accounts[1].key {
        return Err(ProgramError::InvalidSeeds);
    }

    let signer_seeds: &[&[u8]] = &[TOKEN_ACCOUNT_SEED, &[bump]];
    RevokeCpi {
        token_account: accounts[0].clone(),
        owner: accounts[1].clone(),
        system_program: accounts[2].clone(),
        max_top_up: None,
    }
    .invoke_signed(&[signer_seeds])?;

    Ok(())
}
