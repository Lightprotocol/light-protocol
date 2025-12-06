use borsh::{BorshDeserialize, BorshSerialize};
use light_ctoken_sdk::ctoken::{TransferCtokenToSplCpi, TransferSplToCtokenCpi};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::ID;

/// PDA seed for authority in invoke_signed variants
pub const TRANSFER_AUTHORITY_SEED: &[u8] = b"transfer_authority";

/// Instruction data for SPL to CToken transfer
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TransferSplToCtokenData {
    pub amount: u64,
    pub spl_interface_pda_bump: u8,
}

/// Instruction data for CToken to SPL transfer
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TransferCtokenToSplData {
    pub amount: u64,
    pub spl_interface_pda_bump: u8,
}

/// Handler for transferring SPL tokens to CToken (invoke)
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
pub fn process_spl_to_ctoken_invoke(
    accounts: &[AccountInfo],
    data: TransferSplToCtokenData,
) -> Result<(), ProgramError> {
    if accounts.len() < 9 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    TransferSplToCtokenCpi {
        source_spl_token_account: accounts[1].clone(),
        destination_ctoken_account: accounts[2].clone(),
        amount: data.amount,
        authority: accounts[3].clone(),
        mint: accounts[4].clone(),
        payer: accounts[5].clone(),
        spl_interface_pda: accounts[6].clone(),
        spl_interface_pda_bump: data.spl_interface_pda_bump,
        spl_token_program: accounts[7].clone(),
        compressed_token_program_authority: accounts[8].clone(),
    }
    .invoke()?;

    Ok(())
}

/// Handler for transferring SPL tokens to CToken with PDA authority (invoke_signed)
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
pub fn process_spl_to_ctoken_invoke_signed(
    accounts: &[AccountInfo],
    data: TransferSplToCtokenData,
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

    let account_infos = TransferSplToCtokenCpi {
        source_spl_token_account: accounts[1].clone(),
        destination_ctoken_account: accounts[2].clone(),
        amount: data.amount,
        authority: accounts[3].clone(),
        mint: accounts[4].clone(),
        payer: accounts[5].clone(),
        spl_interface_pda: accounts[6].clone(),
        spl_interface_pda_bump: data.spl_interface_pda_bump,
        spl_token_program: accounts[7].clone(),
        compressed_token_program_authority: accounts[8].clone(),
    };

    // Invoke with PDA signing
    let authority_seeds: &[&[u8]] = &[TRANSFER_AUTHORITY_SEED, &[authority_bump]];
    account_infos.invoke_signed(&[authority_seeds])?;

    Ok(())
}

/// Handler for transferring CToken to SPL tokens (invoke)
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
    data: TransferCtokenToSplData,
) -> Result<(), ProgramError> {
    if accounts.len() < 9 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    TransferCtokenToSplCpi {
        source_ctoken_account: accounts[1].clone(),
        destination_spl_token_account: accounts[2].clone(),
        amount: data.amount,
        authority: accounts[3].clone(),
        mint: accounts[4].clone(),
        payer: accounts[5].clone(),
        spl_interface_pda: accounts[6].clone(),
        spl_interface_pda_bump: data.spl_interface_pda_bump,
        spl_token_program: accounts[7].clone(),
        compressed_token_program_authority: accounts[8].clone(),
    }
    .invoke()?;

    Ok(())
}

/// Handler for transferring CToken to SPL tokens with PDA authority (invoke_signed)
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
    data: TransferCtokenToSplData,
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

    let account_infos = TransferCtokenToSplCpi {
        source_ctoken_account: accounts[1].clone(),
        destination_spl_token_account: accounts[2].clone(),
        amount: data.amount,
        authority: accounts[3].clone(),
        mint: accounts[4].clone(),
        payer: accounts[5].clone(),
        spl_interface_pda: accounts[6].clone(),
        spl_interface_pda_bump: data.spl_interface_pda_bump,
        spl_token_program: accounts[7].clone(),
        compressed_token_program_authority: accounts[8].clone(),
    };

    // Invoke with PDA signing
    let authority_seeds: &[&[u8]] = &[TRANSFER_AUTHORITY_SEED, &[authority_bump]];
    account_infos.invoke_signed(&[authority_seeds])?;

    Ok(())
}
