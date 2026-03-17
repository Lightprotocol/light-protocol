use borsh::{BorshDeserialize, BorshSerialize};
use light_token_pinocchio::instruction::ApproveCpi;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
};

use crate::{ID, TOKEN_ACCOUNT_SEED};

/// Instruction data for approve operations
#[derive(BorshSerialize, BorshDeserialize)]
pub struct ApproveData {
    pub amount: u64,
}

/// Handler for approving a delegate for a Light Token account (invoke)
///
/// Account order:
/// - accounts[0]: token_account (writable)
/// - accounts[1]: delegate
/// - accounts[2]: owner (signer)
/// - accounts[3]: system_program
/// - accounts[4]: light_token_program
pub fn process_approve_invoke(
    accounts: &[AccountInfo],
    data: ApproveData,
) -> Result<(), ProgramError> {
    if accounts.len() < 5 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    ApproveCpi {
        token_account: &accounts[0],
        delegate: &accounts[1],
        owner: &accounts[2],
        system_program: &accounts[3],
        amount: data.amount,
        fee_payer: &accounts[2],
    }
    .invoke()?;

    Ok(())
}

/// Handler for approving a delegate for a PDA-owned Light Token account (invoke_signed)
///
/// Account order:
/// - accounts[0]: token_account (writable)
/// - accounts[1]: delegate
/// - accounts[2]: PDA owner (program signs)
/// - accounts[3]: system_program
/// - accounts[4]: light_token_program
/// - accounts[5]: fee_payer (writable, signer)
pub fn process_approve_invoke_signed(
    accounts: &[AccountInfo],
    data: ApproveData,
) -> Result<(), ProgramError> {
    if accounts.len() < 6 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the owner
    let (pda, bump) = pinocchio::pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &ID);

    // Verify the owner account is the PDA we expect
    if pda != *accounts[2].key() {
        return Err(ProgramError::InvalidSeeds);
    }
    let bump_byte = [bump];
    let seeds = [Seed::from(TOKEN_ACCOUNT_SEED), Seed::from(&bump_byte[..])];
    let signer = Signer::from(&seeds);
    ApproveCpi {
        token_account: &accounts[0],
        delegate: &accounts[1],
        owner: &accounts[2],
        system_program: &accounts[3],
        amount: data.amount,
        fee_payer: &accounts[5],
    }
    .invoke_signed(&[signer])?;

    Ok(())
}

/// Handler for approving a delegate with a separate fee_payer (invoke)
///
/// Account order:
/// - accounts[0]: token_account (writable)
/// - accounts[1]: delegate
/// - accounts[2]: owner (signer)
/// - accounts[3]: system_program
/// - accounts[4]: light_token_program
/// - accounts[5]: fee_payer (writable, signer)
pub fn process_approve_invoke_with_fee_payer(
    accounts: &[AccountInfo],
    data: ApproveData,
) -> Result<(), ProgramError> {
    if accounts.len() < 6 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    ApproveCpi {
        token_account: &accounts[0],
        delegate: &accounts[1],
        owner: &accounts[2],
        system_program: &accounts[3],
        amount: data.amount,
        fee_payer: &accounts[5],
    }
    .invoke()?;

    Ok(())
}
