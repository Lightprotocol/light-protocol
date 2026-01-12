use borsh::{BorshDeserialize, BorshSerialize};
use light_token_sdk::token::{CompressibleParamsCpi, CreateTokenAccountCpi};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{ID, TOKEN_ACCOUNT_SEED};

/// Instruction data for create token account
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateTokenAccountData {
    pub owner: Pubkey,
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

    // Build the compressible params using constructor
    let compressible_params = CompressibleParamsCpi::new(
        accounts[3].clone(),
        accounts[5].clone(),
        accounts[4].clone(),
    );

    // Build the account infos struct
    CreateTokenAccountCpi {
        payer: accounts[0].clone(),
        account: accounts[1].clone(),
        mint: accounts[2].clone(),
        owner: data.owner,
        compressible: compressible_params,
    }
    .invoke()?;

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
    let (pda, bump) = Pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &ID);

    // Verify the account to create is the PDA
    if &pda != accounts[1].key {
        return Err(ProgramError::InvalidSeeds);
    }

    // Build the compressible params using constructor
    let compressible_params = CompressibleParamsCpi::new(
        accounts[3].clone(),
        accounts[5].clone(),
        accounts[4].clone(),
    );

    // Build the account infos struct
    let account_infos = CreateTokenAccountCpi {
        payer: accounts[0].clone(),
        account: accounts[1].clone(),
        mint: accounts[2].clone(),
        owner: data.owner,
        compressible: compressible_params,
    };

    // Invoke with PDA signing
    let signer_seeds: &[&[u8]] = &[TOKEN_ACCOUNT_SEED, &[bump]];
    account_infos.invoke_signed(&[signer_seeds])?;

    Ok(())
}
