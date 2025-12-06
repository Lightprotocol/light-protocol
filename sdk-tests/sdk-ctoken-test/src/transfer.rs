use borsh::{BorshDeserialize, BorshSerialize};
use light_ctoken_sdk::ctoken::TransferCtokenCpi;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

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
pub fn process_transfer_invoke(
    accounts: &[AccountInfo],
    data: TransferData,
) -> Result<(), ProgramError> {
    if accounts.len() < 3 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Build the account infos struct using the builder pattern
    TransferCtokenCpi {
        source: accounts[0].clone(),
        destination: accounts[1].clone(),
        amount: data.amount,
        authority: accounts[2].clone(),
        max_top_up: None,
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
pub fn process_transfer_invoke_signed(
    accounts: &[AccountInfo],
    data: TransferData,
) -> Result<(), ProgramError> {
    if accounts.len() < 3 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the authority
    let (pda, bump) = Pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &ID);

    // Verify the authority account is the PDA we expect
    if &pda != accounts[2].key {
        return Err(ProgramError::InvalidSeeds);
    }

    // Build the account infos struct
    let transfer_accounts = TransferCtokenCpi {
        source: accounts[0].clone(),
        destination: accounts[1].clone(),
        amount: data.amount,
        authority: accounts[2].clone(),
        max_top_up: None,
    };

    // Invoke with PDA signing - the builder handles instruction creation and invoke_signed CPI
    let signer_seeds: &[&[u8]] = &[TOKEN_ACCOUNT_SEED, &[bump]];
    transfer_accounts.invoke_signed(&[signer_seeds])?;

    Ok(())
}
