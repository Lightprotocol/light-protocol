use light_token_sdk::token::CloseAccountCpi;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{ID, TOKEN_ACCOUNT_SEED};

/// Handler for closing a compressed token account (invoke)
///
/// Account order:
/// - accounts[0]: token_program (ctoken program)
/// - accounts[1]: account to close (writable)
/// - accounts[2]: destination for lamports (writable)
/// - accounts[3]: owner/authority (signer)
/// - accounts[4]: rent_sponsor (writable)
pub fn process_close_account_invoke(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    if accounts.len() < 5 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    CloseAccountCpi {
        token_program: accounts[0].clone(),
        account: accounts[1].clone(),
        destination: accounts[2].clone(),
        owner: accounts[3].clone(),
        rent_sponsor: accounts[4].clone(),
    }
    .invoke()?;

    Ok(())
}

/// Handler for closing a PDA-owned compressed token account (invoke_signed)
///
/// Account order:
/// - accounts[0]: token_program (ctoken program)
/// - accounts[1]: account to close (writable)
/// - accounts[2]: destination for lamports (writable)
/// - accounts[3]: PDA owner/authority (not signer, program signs)
/// - accounts[4]: rent_sponsor (writable)
pub fn process_close_account_invoke_signed(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    if accounts.len() < 5 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the authority
    let (pda, bump) = Pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &ID);

    // Verify the authority account is the PDA we expect
    if &pda != accounts[3].key {
        return Err(ProgramError::InvalidSeeds);
    }

    let signer_seeds: &[&[u8]] = &[TOKEN_ACCOUNT_SEED, &[bump]];
    CloseAccountCpi {
        token_program: accounts[0].clone(),
        account: accounts[1].clone(),
        destination: accounts[2].clone(),
        owner: accounts[3].clone(),
        rent_sponsor: accounts[4].clone(),
    }
    .invoke_signed(&[signer_seeds])?;

    Ok(())
}
