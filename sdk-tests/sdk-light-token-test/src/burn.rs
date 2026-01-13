use borsh::{BorshDeserialize, BorshSerialize};
use light_token_sdk::token::BurnCpi;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

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
/// - accounts[1]: cmint (writable)
/// - accounts[2]: authority (owner, signer)
/// - accounts[3]: ctoken_program
pub fn process_burn_invoke(accounts: &[AccountInfo], amount: u64) -> Result<(), ProgramError> {
    if accounts.len() < 4 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    BurnCpi {
        source: accounts[0].clone(),
        cmint: accounts[1].clone(),
        amount,
        authority: accounts[2].clone(),
        max_top_up: None,
    }
    .invoke()?;

    Ok(())
}

/// Handler for burning CTokens with PDA authority (invoke_signed)
///
/// Account order:
/// - accounts[0]: source (Light Token account, writable)
/// - accounts[1]: cmint (writable)
/// - accounts[2]: PDA authority (owner, program signs)
/// - accounts[3]: ctoken_program
pub fn process_burn_invoke_signed(
    accounts: &[AccountInfo],
    amount: u64,
) -> Result<(), ProgramError> {
    if accounts.len() < 4 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the token account owner
    let (pda, bump) = Pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &ID);

    // Verify the authority account is the PDA we expect
    if &pda != accounts[2].key {
        return Err(ProgramError::InvalidSeeds);
    }

    let signer_seeds: &[&[u8]] = &[TOKEN_ACCOUNT_SEED, &[bump]];
    BurnCpi {
        source: accounts[0].clone(),
        cmint: accounts[1].clone(),
        amount,
        authority: accounts[2].clone(),
        max_top_up: None,
    }
    .invoke_signed(&[signer_seeds])?;

    Ok(())
}
