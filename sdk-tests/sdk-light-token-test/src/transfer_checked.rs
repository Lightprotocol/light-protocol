use borsh::{BorshDeserialize, BorshSerialize};
use light_token::instruction::TransferCheckedCpi;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{ID, TOKEN_ACCOUNT_SEED};

/// Instruction data for transfer_checked operations
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TransferCheckedData {
    pub amount: u64,
    pub decimals: u8,
}

/// Handler for transferring cTokens with checked decimals (invoke)
///
/// Account order:
/// - accounts[0]: source ctoken account
/// - accounts[1]: mint (SPL, T22, or decompressed Mint)
/// - accounts[2]: destination ctoken account
/// - accounts[3]: authority (signer)
pub fn process_transfer_checked_invoke(
    accounts: &[AccountInfo],
    data: TransferCheckedData,
) -> Result<(), ProgramError> {
    if accounts.len() < 4 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    TransferCheckedCpi {
        source: accounts[0].clone(),
        mint: accounts[1].clone(),
        destination: accounts[2].clone(),
        amount: data.amount,
        decimals: data.decimals,
        authority: accounts[3].clone(),
        max_top_up: None,
    }
    .invoke()?;

    Ok(())
}

/// Handler for transferring cTokens with checked decimals from PDA-owned account (invoke_signed)
///
/// Account order:
/// - accounts[0]: source ctoken account (PDA-owned)
/// - accounts[1]: mint (SPL, T22, or decompressed Mint)
/// - accounts[2]: destination ctoken account
/// - accounts[3]: authority (PDA)
pub fn process_transfer_checked_invoke_signed(
    accounts: &[AccountInfo],
    data: TransferCheckedData,
) -> Result<(), ProgramError> {
    if accounts.len() < 4 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the authority
    let (pda, bump) = Pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &ID);

    // Verify the authority account is the PDA we expect
    if &pda != accounts[3].key {
        return Err(ProgramError::InvalidSeeds);
    }

    let transfer_accounts = TransferCheckedCpi {
        source: accounts[0].clone(),
        mint: accounts[1].clone(),
        destination: accounts[2].clone(),
        amount: data.amount,
        decimals: data.decimals,
        authority: accounts[3].clone(),
        max_top_up: None,
    };

    // Invoke with PDA signing
    let signer_seeds: &[&[u8]] = &[TOKEN_ACCOUNT_SEED, &[bump]];
    transfer_accounts.invoke_signed(&[signer_seeds])?;

    Ok(())
}
