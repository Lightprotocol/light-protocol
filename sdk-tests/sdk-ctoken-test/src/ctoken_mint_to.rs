use borsh::{BorshDeserialize, BorshSerialize};
use light_ctoken_sdk::ctoken::CTokenMintToCpi;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{mint_to_ctoken::MINT_AUTHORITY_SEED, ID};

/// Instruction data for CTokenMintTo operations
#[derive(BorshSerialize, BorshDeserialize)]
pub struct MintToData {
    pub amount: u64,
}

/// Handler for minting to CToken (invoke)
///
/// Account order:
/// - accounts[0]: cmint (writable)
/// - accounts[1]: destination (CToken account, writable)
/// - accounts[2]: authority (mint authority, signer)
/// - accounts[3]: ctoken_program
pub fn process_ctoken_mint_to_invoke(
    accounts: &[AccountInfo],
    amount: u64,
) -> Result<(), ProgramError> {
    if accounts.len() < 4 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    CTokenMintToCpi {
        cmint: accounts[0].clone(),
        destination: accounts[1].clone(),
        amount,
        authority: accounts[2].clone(),
        max_top_up: None,
    }
    .invoke()?;

    Ok(())
}

/// Handler for minting to CToken with PDA authority (invoke_signed)
///
/// Account order:
/// - accounts[0]: cmint (writable)
/// - accounts[1]: destination (CToken account, writable)
/// - accounts[2]: PDA authority (mint authority, program signs)
/// - accounts[3]: ctoken_program
pub fn process_ctoken_mint_to_invoke_signed(
    accounts: &[AccountInfo],
    amount: u64,
) -> Result<(), ProgramError> {
    if accounts.len() < 4 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the mint authority
    let (pda, bump) = Pubkey::find_program_address(&[MINT_AUTHORITY_SEED], &ID);

    // Verify the authority account is the PDA we expect
    if &pda != accounts[2].key {
        return Err(ProgramError::InvalidSeeds);
    }

    let signer_seeds: &[&[u8]] = &[MINT_AUTHORITY_SEED, &[bump]];
    CTokenMintToCpi {
        cmint: accounts[0].clone(),
        destination: accounts[1].clone(),
        amount,
        authority: accounts[2].clone(),
        max_top_up: None,
    }
    .invoke_signed(&[signer_seeds])?;

    Ok(())
}
