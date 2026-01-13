use borsh::{BorshDeserialize, BorshSerialize};
use light_token_sdk::token::ApproveCpi;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{ID, TOKEN_ACCOUNT_SEED};

/// Instruction data for approve operations
#[derive(BorshSerialize, BorshDeserialize)]
pub struct ApproveData {
    pub amount: u64,
}

/// Handler for approving a delegate for a CToken account (invoke)
///
/// Account order:
/// - accounts[0]: token_account (writable)
/// - accounts[1]: delegate
/// - accounts[2]: owner (signer)
/// - accounts[3]: system_program
/// - accounts[4]: ctoken_program
pub fn process_approve_invoke(
    accounts: &[AccountInfo],
    data: ApproveData,
) -> Result<(), ProgramError> {
    if accounts.len() < 5 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    ApproveCpi {
        token_account: accounts[0].clone(),
        delegate: accounts[1].clone(),
        owner: accounts[2].clone(),
        system_program: accounts[3].clone(),
        amount: data.amount,
    }
    .invoke()?;

    Ok(())
}

/// Handler for approving a delegate for a PDA-owned CToken account (invoke_signed)
///
/// Account order:
/// - accounts[0]: token_account (writable)
/// - accounts[1]: delegate
/// - accounts[2]: PDA owner (program signs)
/// - accounts[3]: system_program
/// - accounts[4]: ctoken_program
pub fn process_approve_invoke_signed(
    accounts: &[AccountInfo],
    data: ApproveData,
) -> Result<(), ProgramError> {
    if accounts.len() < 5 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the owner
    let (pda, bump) = Pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &ID);

    // Verify the owner account is the PDA we expect
    if &pda != accounts[2].key {
        return Err(ProgramError::InvalidSeeds);
    }

    let signer_seeds: &[&[u8]] = &[TOKEN_ACCOUNT_SEED, &[bump]];
    ApproveCpi {
        token_account: accounts[0].clone(),
        delegate: accounts[1].clone(),
        owner: accounts[2].clone(),
        system_program: accounts[3].clone(),
        amount: data.amount,
    }
    .invoke_signed(&[signer_seeds])?;

    Ok(())
}
