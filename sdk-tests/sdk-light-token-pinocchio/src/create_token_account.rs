use borsh::{BorshDeserialize, BorshSerialize};
use light_token_pinocchio::instruction::{CompressibleParamsCpi, CreateTokenAccountCpi};
use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    AccountView as AccountInfo, Address,
};

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
    let (pda, bump) = Address::find_program_address(&[TOKEN_ACCOUNT_SEED], &Address::from(ID));

    // Verify the account to create is the PDA
    if pda != *accounts[1].address() {
        return Err(ProgramError::InvalidSeeds);
    }

    // Invoke with PDA signing and rent-free config
    let bump_byte = [bump];
    let seeds = [Seed::from(TOKEN_ACCOUNT_SEED), Seed::from(&bump_byte[..])];
    let signer = Signer::from(&seeds);
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
    )
    .invoke_signed(&[signer])
    .map_err(|_| ProgramError::Custom(0))?;

    Ok(())
}

/// Handler for creating a compressible token account using invoke_with (explicit CompressibleParamsCpi).
///
/// Account order:
/// - accounts[0]: payer (signer)
/// - accounts[1]: account to create (signer)
/// - accounts[2]: mint
/// - accounts[3]: compressible_config
/// - accounts[4]: system_program
/// - accounts[5]: rent_sponsor
pub fn process_create_token_account_invoke_with(
    accounts: &[AccountInfo],
    data: CreateTokenAccountData,
) -> Result<(), ProgramError> {
    if accounts.len() < 6 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let compressible = CompressibleParamsCpi::new(
        &accounts[3], // compressible_config
        &accounts[5], // rent_sponsor
        &accounts[4], // system_program
    );

    CreateTokenAccountCpi {
        payer: &accounts[0],
        account: &accounts[1],
        mint: &accounts[2],
        owner: data.owner,
    }
    .invoke_with(compressible)
    .map_err(|_| ProgramError::Custom(0))?;

    Ok(())
}

/// Handler for creating a PDA-owned compressible token account using invoke_signed_with.
///
/// Account order:
/// - accounts[0]: payer (signer)
/// - accounts[1]: account to create (PDA, will be derived and verified)
/// - accounts[2]: mint
/// - accounts[3]: compressible_config
/// - accounts[4]: system_program
/// - accounts[5]: rent_sponsor
pub fn process_create_token_account_invoke_signed_with(
    accounts: &[AccountInfo],
    data: CreateTokenAccountData,
) -> Result<(), ProgramError> {
    if accounts.len() < 6 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let (pda, bump) = Address::find_program_address(&[TOKEN_ACCOUNT_SEED], &Address::from(ID));

    if pda != *accounts[1].address() {
        return Err(ProgramError::InvalidSeeds);
    }

    let bump_byte = [bump];
    let seeds = [Seed::from(TOKEN_ACCOUNT_SEED), Seed::from(&bump_byte[..])];
    let signer = Signer::from(&seeds);

    let compressible = CompressibleParamsCpi::new(
        &accounts[3], // compressible_config
        &accounts[5], // rent_sponsor
        &accounts[4], // system_program
    );

    CreateTokenAccountCpi {
        payer: &accounts[0],
        account: &accounts[1],
        mint: &accounts[2],
        owner: data.owner,
    }
    .invoke_signed_with(compressible, &[signer])
    .map_err(|_| ProgramError::Custom(0))?;

    Ok(())
}
