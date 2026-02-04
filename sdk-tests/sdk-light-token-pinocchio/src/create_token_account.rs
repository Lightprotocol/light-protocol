use borsh::{BorshDeserialize, BorshSerialize};
use light_token_pinocchio::instruction::CreateTokenAccountCpi;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

use crate::{ID, TOKEN_ACCOUNT_SEED};

/// Instruction data for create token account
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateTokenAccountData {
    pub owner: [u8; 32],
    pub pre_pay_num_epochs: u8,
    pub lamports_per_write: u32,
}

/// Handler for creating a compressible token account (invoke)
///
/// Uses the builder pattern from the ctoken module. This demonstrates how to:
/// 1. Build the account infos struct with compressible params
/// 2. Call the invoke() method which handles instruction building and CPI
///
/// Account order:
/// - accounts[0]: payer (signer)
/// - accounts[1]: account to create (signer)
/// - accounts[2]: mint
/// - accounts[3]: compressible_config
/// - accounts[4]: system_program
/// - accounts[5]: rent_sponsor
pub fn process_create_token_account_invoke(
    accounts: &[AccountInfo],
    data: CreateTokenAccountData,
) -> Result<(), ProgramError> {
    if accounts.len() < 6 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Build the account infos struct and invoke with rent-free config
    CreateTokenAccountCpi {
        payer: &accounts[0],
        account: &accounts[1],
        mint: &accounts[2],
        owner: data.owner,
    }
    .rent_free(
        &accounts[3], // compressible_config
        &accounts[5], // rent_sponsor
        &accounts[4], // system_program
        &ID,
    )
    .invoke()
    .map_err(|_| ProgramError::Custom(0))?;

    Ok(())
}

/// Handler for creating a compressible token account with PDA ownership (invoke_signed)
///
/// Account order:
/// - accounts[0]: payer (signer)
/// - accounts[1]: account to create (PDA, will be derived and verified)
/// - accounts[2]: mint
/// - accounts[3]: compressible_config
/// - accounts[4]: system_program
/// - accounts[5]: rent_sponsor
pub fn process_create_token_account_invoke_signed(
    accounts: &[AccountInfo],
    data: CreateTokenAccountData,
) -> Result<(), ProgramError> {
    if accounts.len() < 6 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the token account
    let (pda, bump) = pinocchio::pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &ID);

    // Verify the account to create is the PDA
    if pda != *accounts[1].key() {
        return Err(ProgramError::InvalidSeeds);
    }

    // Invoke with PDA signing and rent-free config
    let signer_seeds: &[&[u8]] = &[TOKEN_ACCOUNT_SEED, &[bump]];
    CreateTokenAccountCpi {
        payer: &accounts[0],
        account: &accounts[1],
        mint: &accounts[2],
        owner: data.owner,
    }
    .rent_free(
        &accounts[3], // compressible_config
        &accounts[5], // rent_sponsor
        &accounts[4], // system_program
        &ID,
    )
    .invoke_signed(signer_seeds)
    .map_err(|_| ProgramError::Custom(0))?;

    Ok(())
}
