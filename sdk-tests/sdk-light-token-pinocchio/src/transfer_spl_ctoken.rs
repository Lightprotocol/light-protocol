use borsh::{BorshDeserialize, BorshSerialize};
use light_token_pinocchio::instruction::{TransferFromSplCpi, TransferToSplCpi};
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
};

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
/// - accounts[2]: destination (writable)
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
        source_spl_token_account: &accounts[1],
        destination: &accounts[2],
        amount: data.amount,
        authority: &accounts[3],
        mint: &accounts[4],
        payer: &accounts[5],
        spl_interface_pda: &accounts[6],
        spl_interface_pda_bump: data.spl_interface_pda_bump,
        decimals: data.decimals,
        spl_token_program: &accounts[7],
        compressed_token_program_authority: &accounts[8],
        system_program: &accounts[9],
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
/// - accounts[2]: destination (writable)
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
        pinocchio::pubkey::find_program_address(&[TRANSFER_AUTHORITY_SEED], &ID);

    // Verify the authority account is the PDA we expect
    if authority_pda != *accounts[3].key() {
        return Err(ProgramError::InvalidSeeds);
    }

    let account_infos = TransferFromSplCpi {
        source_spl_token_account: &accounts[1],
        destination: &accounts[2],
        amount: data.amount,
        authority: &accounts[3],
        mint: &accounts[4],
        payer: &accounts[5],
        spl_interface_pda: &accounts[6],
        spl_interface_pda_bump: data.spl_interface_pda_bump,
        decimals: data.decimals,
        spl_token_program: &accounts[7],
        compressed_token_program_authority: &accounts[8],
        system_program: &accounts[9],
    };

    // Invoke with PDA signing
    let authority_bump_byte = [authority_bump];
    let authority_seeds = [
        Seed::from(TRANSFER_AUTHORITY_SEED),
        Seed::from(&authority_bump_byte[..]),
    ];
    let authority_signer = Signer::from(&authority_seeds);
    account_infos.invoke_signed(&[authority_signer])?;

    Ok(())
}

/// Handler for transferring Light Token to SPL tokens (invoke)
///
/// Account order:
/// - accounts[0]: compressed_token_program (for CPI)
/// - accounts[1]: source
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
        source: &accounts[1],
        destination_spl_token_account: &accounts[2],
        amount: data.amount,
        authority: &accounts[3],
        mint: &accounts[4],
        payer: &accounts[5],
        spl_interface_pda: &accounts[6],
        spl_interface_pda_bump: data.spl_interface_pda_bump,
        decimals: data.decimals,
        spl_token_program: &accounts[7],
        compressed_token_program_authority: &accounts[8],
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
/// - accounts[1]: source
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
        pinocchio::pubkey::find_program_address(&[TRANSFER_AUTHORITY_SEED], &ID);

    // Verify the authority account is the PDA we expect
    if authority_pda != *accounts[3].key() {
        return Err(ProgramError::InvalidSeeds);
    }

    let account_infos = TransferToSplCpi {
        source: &accounts[1],
        destination_spl_token_account: &accounts[2],
        amount: data.amount,
        authority: &accounts[3],
        mint: &accounts[4],
        payer: &accounts[5],
        spl_interface_pda: &accounts[6],
        spl_interface_pda_bump: data.spl_interface_pda_bump,
        decimals: data.decimals,
        spl_token_program: &accounts[7],
        compressed_token_program_authority: &accounts[8],
    };

    // Invoke with PDA signing
    let authority_bump_byte = [authority_bump];
    let authority_seeds = [
        Seed::from(TRANSFER_AUTHORITY_SEED),
        Seed::from(&authority_bump_byte[..]),
    ];
    let authority_signer = Signer::from(&authority_seeds);
    account_infos.invoke_signed(&[authority_signer])?;

    Ok(())
}
