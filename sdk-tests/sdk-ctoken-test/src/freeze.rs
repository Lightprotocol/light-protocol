use light_ctoken_sdk::ctoken::FreezeCTokenCpi;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{FREEZE_AUTHORITY_SEED, ID};

/// Handler for freezing a CToken account (invoke)
///
/// Account order:
/// - accounts[0]: token_account (writable)
/// - accounts[1]: mint
/// - accounts[2]: freeze_authority (signer)
/// - accounts[3]: ctoken_program
pub fn process_freeze_invoke(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    if accounts.len() < 4 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    FreezeCTokenCpi {
        token_account: accounts[0].clone(),
        mint: accounts[1].clone(),
        freeze_authority: accounts[2].clone(),
    }
    .invoke()?;

    Ok(())
}

/// Handler for freezing a CToken account with PDA freeze authority (invoke_signed)
///
/// Account order:
/// - accounts[0]: token_account (writable)
/// - accounts[1]: mint
/// - accounts[2]: PDA freeze_authority (program signs)
/// - accounts[3]: ctoken_program
pub fn process_freeze_invoke_signed(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    if accounts.len() < 4 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the freeze authority
    let (pda, bump) = Pubkey::find_program_address(&[FREEZE_AUTHORITY_SEED], &ID);

    // Verify the freeze_authority account is the PDA we expect
    if &pda != accounts[2].key {
        return Err(ProgramError::InvalidSeeds);
    }

    let signer_seeds: &[&[u8]] = &[FREEZE_AUTHORITY_SEED, &[bump]];
    FreezeCTokenCpi {
        token_account: accounts[0].clone(),
        mint: accounts[1].clone(),
        freeze_authority: accounts[2].clone(),
    }
    .invoke_signed(&[signer_seeds])?;

    Ok(())
}
