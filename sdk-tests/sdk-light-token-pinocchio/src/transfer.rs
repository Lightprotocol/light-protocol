use borsh::{BorshDeserialize, BorshSerialize};
use light_token_pinocchio::instruction::TransferCpi;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
};

use crate::{ID, TOKEN_ACCOUNT_SEED};

/// Instruction data for transfer operations
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TransferData {
    pub amount: u64,
}

/// Handler for transferring compressed tokens (invoke)
///
/// Uses the builder pattern from the ctoken module. This demonstrates how to:
/// 1. Build the account infos struct
/// 2. Call the invoke() method which handles instruction building and CPI
///
/// Account order:
/// - accounts[0]: source ctoken account
/// - accounts[1]: destination ctoken account
/// - accounts[2]: authority (signer)
/// - accounts[3]: system_program
pub fn process_transfer_invoke(
    accounts: &[AccountInfo],
    data: TransferData,
) -> Result<(), ProgramError> {
    if accounts.len() < 4 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Build the account infos struct using the builder pattern
    TransferCpi {
        source: &accounts[0],
        destination: &accounts[1],
        amount: data.amount,
        authority: &accounts[2],
        system_program: &accounts[3],
        fee_payer: None,
    }
    .invoke()?;

    Ok(())
}

/// Handler for transferring compressed tokens from PDA-owned account (invoke_signed)
///
/// Uses the builder pattern with invoke_signed. This demonstrates how to:
/// 1. Build the account infos struct
/// 2. Derive PDA seeds
/// 3. Call invoke_signed() method with the signer seeds
///
/// Account order:
/// - accounts[0]: source ctoken account (PDA-owned)
/// - accounts[1]: destination ctoken account
/// - accounts[2]: authority (PDA)
/// - accounts[3]: system_program
pub fn process_transfer_invoke_signed(
    accounts: &[AccountInfo],
    data: TransferData,
) -> Result<(), ProgramError> {
    if accounts.len() < 4 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the authority
    let (pda, bump) = pinocchio::pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &ID);

    // Verify the authority account is the PDA we expect
    if pda != *accounts[2].key() {
        return Err(ProgramError::InvalidSeeds);
    }

    // Build the account infos struct
    let transfer_accounts = TransferCpi {
        source: &accounts[0],
        destination: &accounts[1],
        amount: data.amount,
        authority: &accounts[2],
        system_program: &accounts[3],
        fee_payer: None,
    };

    // Invoke with PDA signing - the builder handles instruction creation and invoke_signed CPI
    let bump_byte = [bump];
    let seeds = [Seed::from(TOKEN_ACCOUNT_SEED), Seed::from(&bump_byte[..])];
    let signer = Signer::from(&seeds);
    transfer_accounts.invoke_signed(&[signer])?;

    Ok(())
}
