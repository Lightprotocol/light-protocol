use borsh::{BorshDeserialize, BorshSerialize};
use light_token_pinocchio::instruction::TransferCheckedCpi;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
};

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
/// - accounts[4]: system_program
pub fn process_transfer_checked_invoke(
    accounts: &[AccountInfo],
    data: TransferCheckedData,
) -> Result<(), ProgramError> {
    if accounts.len() < 5 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    TransferCheckedCpi {
        source: &accounts[0],
        mint: &accounts[1],
        destination: &accounts[2],
        amount: data.amount,
        decimals: data.decimals,
        authority: &accounts[3],
        system_program: &accounts[4],
        fee_payer: None,
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
/// - accounts[4]: system_program
pub fn process_transfer_checked_invoke_signed(
    accounts: &[AccountInfo],
    data: TransferCheckedData,
) -> Result<(), ProgramError> {
    if accounts.len() < 5 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the authority
    let (pda, bump) = pinocchio::pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &ID);

    // Verify the authority account is the PDA we expect
    if pda != *accounts[3].key() {
        return Err(ProgramError::InvalidSeeds);
    }

    let transfer_accounts = TransferCheckedCpi {
        source: &accounts[0],
        mint: &accounts[1],
        destination: &accounts[2],
        amount: data.amount,
        decimals: data.decimals,
        authority: &accounts[3],
        system_program: &accounts[4],
        fee_payer: None,
    };

    // Invoke with PDA signing
    let bump_byte = [bump];
    let seeds = [Seed::from(TOKEN_ACCOUNT_SEED), Seed::from(&bump_byte[..])];
    let signer = Signer::from(&seeds);
    transfer_accounts.invoke_signed(&[signer])?;

    Ok(())
}
