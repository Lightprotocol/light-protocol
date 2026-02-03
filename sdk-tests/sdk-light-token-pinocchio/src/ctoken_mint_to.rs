use borsh::{BorshDeserialize, BorshSerialize};
use light_token_pinocchio::instruction::MintToCpi;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
};

use crate::{ID, MINT_AUTHORITY_SEED};

/// Instruction data for MintTo operations
#[derive(BorshSerialize, BorshDeserialize)]
pub struct MintToData {
    pub amount: u64,
}

/// Handler for minting to Token (invoke)
///
/// Account order:
/// - accounts[0]: mint (writable)
/// - accounts[1]: destination (Token account, writable)
/// - accounts[2]: authority (mint authority, signer)
/// - accounts[3]: system_program
/// - accounts[4]: light_token_program
pub fn process_mint_to_invoke(accounts: &[AccountInfo], amount: u64) -> Result<(), ProgramError> {
    if accounts.len() < 5 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    MintToCpi {
        mint: &accounts[0],
        destination: &accounts[1],
        amount,
        authority: &accounts[2],
        system_program: &accounts[3],
        fee_payer: None,
    }
    .invoke()?;

    Ok(())
}

/// Handler for minting to Token with PDA authority (invoke_signed)
///
/// Account order:
/// - accounts[0]: mint (writable)
/// - accounts[1]: destination (Token account, writable)
/// - accounts[2]: PDA authority (mint authority, program signs)
/// - accounts[3]: system_program
/// - accounts[4]: light_token_program
pub fn process_mint_to_invoke_signed(
    accounts: &[AccountInfo],
    amount: u64,
) -> Result<(), ProgramError> {
    if accounts.len() < 5 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the mint authority
    let (pda, bump) = pinocchio::pubkey::find_program_address(&[MINT_AUTHORITY_SEED], &ID);

    // Verify the authority account is the PDA we expect
    if pda != *accounts[2].key() {
        return Err(ProgramError::InvalidSeeds);
    }

    let bump_byte = [bump];
    let seeds = [Seed::from(MINT_AUTHORITY_SEED), Seed::from(&bump_byte[..])];
    let signer = Signer::from(&seeds);

    MintToCpi {
        mint: &accounts[0],
        destination: &accounts[1],
        amount,
        authority: &accounts[2],
        system_program: &accounts[3],
        fee_payer: None,
    }
    .invoke_signed(&[signer])?;

    Ok(())
}
