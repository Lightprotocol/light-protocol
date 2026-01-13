use borsh::{BorshDeserialize, BorshSerialize};
use light_token_sdk::token::{TransferFromSplCpi, TransferToSplCpi};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::ID;

/// PDA seed for authority in invoke_signed variants
pub const TRANSFER_AUTHORITY_SEED: &[u8] = b"transfer_authority";

/// Instruction data for SPL to Light Token transfer
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TransferFromSplData {
    pub amount: u64,
    pub spl_interface_pda_bump: u8,
    pub decimals: u8,
}

/// Instruction data for Light Token to SPL transfer
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TransferTokenToSplData {
    pub amount: u64,
    pub spl_interface_pda_bump: u8,
    pub decimals: u8,
}

/// Handler for transferring SPL tokens to Light Token (invoke)
///
/// Account order:
/// - accounts[0]: compressed_token_program (for CPI)
/// - accounts[1]: source_spl_token_account
/// - accounts[2]: destination_ctoken_account (writable)
/// - accounts[3]: authority (signer)
/// - accounts[4]: mint
/// - accounts[5]: payer (signer)
/// - accounts[6]: spl_interface_pda
/// - accounts[7]: spl_token_program
/// - accounts[8]: compressed_token_program_authority
/// - accounts[9]: system_program
pub fn process_spl_to_ctoken_invoke(
    accounts: &[AccountInfo],
    data: TransferFromSplData,
) -> Result<(), ProgramError> {
    if accounts.len() < 10 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    TransferFromSplCpi {
        source_spl_token_account: accounts[1].clone(),
        destination_ctoken_account: accounts[2].clone(),
        amount: data.amount,
        authority: accounts[3].clone(),
        mint: accounts[4].clone(),
        payer: accounts[5].clone(),
        spl_interface_pda: accounts[6].clone(),
        spl_interface_pda_bump: data.spl_interface_pda_bump,
        decimals: data.decimals,
        spl_token_program: accounts[7].clone(),
        compressed_token_program_authority: accounts[8].clone(),
        system_program: accounts[9].clone(),
    }
    .invoke()?;

    Ok(())
}

/// Handler for transferring SPL tokens to Light Token with PDA authority (invoke_signed)
///
/// The authority is a PDA derived from TRANSFER_AUTHORITY_SEED.
///
/// Account order:
/// - accounts[0]: compressed_token_program (for CPI)
/// - accounts[1]: source_spl_token_account
/// - accounts[2]: destination_ctoken_account (writable)
/// - accounts[3]: authority (PDA, not signer - program signs)
/// - accounts[4]: mint
/// - accounts[5]: payer (signer)
/// - accounts[6]: spl_interface_pda
/// - accounts[7]: spl_token_program
/// - accounts[8]: compressed_token_program_authority
/// - accounts[9]: system_program
pub fn process_spl_to_ctoken_invoke_signed(
    accounts: &[AccountInfo],
    data: TransferFromSplData,
) -> Result<(), ProgramError> {
    if accounts.len() < 10 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the authority
    let (authority_pda, authority_bump) =
        Pubkey::find_program_address(&[TRANSFER_AUTHORITY_SEED], &ID);

    // Verify the authority account is the PDA we expect
    if &authority_pda != accounts[3].key {
        return Err(ProgramError::InvalidSeeds);
    }

    let account_infos = TransferFromSplCpi {
        source_spl_token_account: accounts[1].clone(),
        destination_ctoken_account: accounts[2].clone(),
        amount: data.amount,
        authority: accounts[3].clone(),
        mint: accounts[4].clone(),
        payer: accounts[5].clone(),
        spl_interface_pda: accounts[6].clone(),
        spl_interface_pda_bump: data.spl_interface_pda_bump,
        decimals: data.decimals,
        spl_token_program: accounts[7].clone(),
        compressed_token_program_authority: accounts[8].clone(),
        system_program: accounts[9].clone(),
    };

    // Invoke with PDA signing
    let authority_seeds: &[&[u8]] = &[TRANSFER_AUTHORITY_SEED, &[authority_bump]];
    account_infos.invoke_signed(&[authority_seeds])?;

    Ok(())
}

/// Handler for transferring Light Token to SPL tokens (invoke)
///
/// Account order:
/// - accounts[0]: compressed_token_program (for CPI)
/// - accounts[1]: source_ctoken_account
/// - accounts[2]: destination_spl_token_account
/// - accounts[3]: authority (signer)
/// - accounts[4]: mint
/// - accounts[5]: payer (signer)
/// - accounts[6]: spl_interface_pda
/// - accounts[7]: spl_token_program
/// - accounts[8]: compressed_token_program_authority
pub fn process_ctoken_to_spl_invoke(
    accounts: &[AccountInfo],
    data: TransferTokenToSplData,
) -> Result<(), ProgramError> {
    if accounts.len() < 9 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    TransferToSplCpi {
        source_ctoken_account: accounts[1].clone(),
        destination_spl_token_account: accounts[2].clone(),
        amount: data.amount,
        authority: accounts[3].clone(),
        mint: accounts[4].clone(),
        payer: accounts[5].clone(),
        spl_interface_pda: accounts[6].clone(),
        spl_interface_pda_bump: data.spl_interface_pda_bump,
        decimals: data.decimals,
        spl_token_program: accounts[7].clone(),
        compressed_token_program_authority: accounts[8].clone(),
    }
    .invoke()?;

    Ok(())
}

/// Handler for transferring Light Token to SPL tokens with PDA authority (invoke_signed)
///
/// The authority is a PDA derived from TRANSFER_AUTHORITY_SEED.
///
/// Account order:
/// - accounts[0]: compressed_token_program (for CPI)
/// - accounts[1]: source_ctoken_account
/// - accounts[2]: destination_spl_token_account
/// - accounts[3]: authority (PDA, not signer - program signs)
/// - accounts[4]: mint
/// - accounts[5]: payer (signer)
/// - accounts[6]: spl_interface_pda
/// - accounts[7]: spl_token_program
/// - accounts[8]: compressed_token_program_authority
pub fn process_ctoken_to_spl_invoke_signed(
    accounts: &[AccountInfo],
    data: TransferTokenToSplData,
) -> Result<(), ProgramError> {
    if accounts.len() < 9 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the authority
    let (authority_pda, authority_bump) =
        Pubkey::find_program_address(&[TRANSFER_AUTHORITY_SEED], &ID);

    // Verify the authority account is the PDA we expect
    if &authority_pda != accounts[3].key {
        return Err(ProgramError::InvalidSeeds);
    }

    let account_infos = TransferToSplCpi {
        source_ctoken_account: accounts[1].clone(),
        destination_spl_token_account: accounts[2].clone(),
        amount: data.amount,
        authority: accounts[3].clone(),
        mint: accounts[4].clone(),
        payer: accounts[5].clone(),
        spl_interface_pda: accounts[6].clone(),
        spl_interface_pda_bump: data.spl_interface_pda_bump,
        decimals: data.decimals,
        spl_token_program: accounts[7].clone(),
        compressed_token_program_authority: accounts[8].clone(),
    };

    // Invoke with PDA signing
    let authority_seeds: &[&[u8]] = &[TRANSFER_AUTHORITY_SEED, &[authority_bump]];
    account_infos.invoke_signed(&[authority_seeds])?;

    Ok(())
}
