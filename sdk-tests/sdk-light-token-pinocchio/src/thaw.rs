use light_token_pinocchio::instruction::ThawCpi;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
};

use crate::{FREEZE_AUTHORITY_SEED, ID};

/// Handler for thawing a frozen Light Token account (invoke)
///
/// Account order:
/// - accounts[0]: token_account (writable)
/// - accounts[1]: mint
/// - accounts[2]: freeze_authority (signer)
/// - accounts[3]: light_token_program
pub fn process_thaw_invoke(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    if accounts.len() < 4 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    ThawCpi {
        token_account: &accounts[0],
        mint: &accounts[1],
        freeze_authority: &accounts[2],
    }
    .invoke()?;

    Ok(())
}

/// Handler for thawing a frozen Light Token account with PDA freeze authority (invoke_signed)
///
/// Account order:
/// - accounts[0]: token_account (writable)
/// - accounts[1]: mint
/// - accounts[2]: PDA freeze_authority (program signs)
/// - accounts[3]: light_token_program
pub fn process_thaw_invoke_signed(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    if accounts.len() < 4 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the freeze authority
    let (pda, bump) = pinocchio::pubkey::find_program_address(&[FREEZE_AUTHORITY_SEED], &ID);

    // Verify the freeze_authority account is the PDA we expect
    if pda != *accounts[2].key() {
        return Err(ProgramError::InvalidSeeds);
    }

    let bump_byte = [bump];
    let seeds = [
        Seed::from(FREEZE_AUTHORITY_SEED),
        Seed::from(&bump_byte[..]),
    ];
    let signer = Signer::from(&seeds);

    ThawCpi {
        token_account: &accounts[0],
        mint: &accounts[1],
        freeze_authority: &accounts[2],
    }
    .invoke_signed(&[signer])?;

    Ok(())
}
