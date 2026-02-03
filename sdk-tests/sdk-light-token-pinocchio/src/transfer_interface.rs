use borsh::{BorshDeserialize, BorshSerialize};
use light_token_pinocchio::instruction::{SplInterfaceCpi, TransferInterfaceCpi};
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
};

use crate::ID;

/// PDA seed for authority in invoke_signed variants
pub const TRANSFER_INTERFACE_AUTHORITY_SEED: &[u8] = b"transfer_interface_authority";

/// Instruction data for TransferInterface
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TransferInterfaceData {
    pub amount: u64,
    pub decimals: u8,
    /// Required for SPL<->Light Token transfers, None for Light Token->Light Token
    pub spl_interface_pda_bump: Option<u8>,
}

/// Handler for TransferInterface (invoke)
///
/// This unified interface automatically detects account types and routes to:
/// - Light Token -> Light Token transfer
/// - Light Token -> SPL transfer
/// - SPL -> Light Token transfer
///
/// Account order:
/// - accounts[0]: compressed_token_program (for CPI)
/// - accounts[1]: source_account (SPL or Light Token)
/// - accounts[2]: destination_account (SPL or Light Token)
/// - accounts[3]: authority (signer)
/// - accounts[4]: payer (signer)
/// - accounts[5]: compressed_token_program_authority
/// - accounts[6]: system_program
///   For SPL bridge (optional, required for SPL<->Light Token):
/// - accounts[7]: mint
/// - accounts[8]: spl_interface_pda
/// - accounts[9]: spl_token_program
pub fn process_transfer_interface_invoke(
    accounts: &[AccountInfo],
    data: TransferInterfaceData,
) -> Result<(), ProgramError> {
    if accounts.len() < 7 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let mut transfer = TransferInterfaceCpi::new(
        data.amount,
        data.decimals,
        &accounts[1], // source_account
        &accounts[2], // destination_account
        &accounts[3], // authority
        &accounts[4], // payer
        &accounts[5], // compressed_token_program_authority
        &accounts[6], // system_program
    );

    // Add SPL bridge config if provided
    if accounts.len() >= 10 && data.spl_interface_pda_bump.is_some() {
        transfer = transfer.with_spl_interface(SplInterfaceCpi {
            mint: &accounts[7],
            spl_token_program: &accounts[9],
            spl_interface_pda: &accounts[8],
            spl_interface_pda_bump: data.spl_interface_pda_bump.unwrap(),
        });
    }

    transfer.invoke()?;

    Ok(())
}

/// Handler for TransferInterfaceCpi with PDA authority (invoke_signed)
///
/// The authority is a PDA derived from TRANSFER_INTERFACE_AUTHORITY_SEED.
///
/// Account order:
/// - accounts[0]: compressed_token_program (for CPI)
/// - accounts[1]: source_account (SPL or Light Token)
/// - accounts[2]: destination_account (SPL or Light Token)
/// - accounts[3]: authority (PDA, not signer - program signs)
/// - accounts[4]: payer (signer)
/// - accounts[5]: compressed_token_program_authority
/// - accounts[6]: system_program
///   For SPL bridge (optional, required for SPL<->Light Token):
/// - accounts[7]: mint
/// - accounts[8]: spl_interface_pda
/// - accounts[9]: spl_token_program
pub fn process_transfer_interface_invoke_signed(
    accounts: &[AccountInfo],
    data: TransferInterfaceData,
) -> Result<(), ProgramError> {
    if accounts.len() < 7 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the authority
    let (authority_pda, authority_bump) =
        pinocchio::pubkey::find_program_address(&[TRANSFER_INTERFACE_AUTHORITY_SEED], &ID);

    // Verify the authority account is the PDA we expect
    if authority_pda != *accounts[3].key() {
        return Err(ProgramError::InvalidSeeds);
    }

    let mut transfer = TransferInterfaceCpi::new(
        data.amount,
        data.decimals,
        &accounts[1], // source_account
        &accounts[2], // destination_account
        &accounts[3], // authority (PDA)
        &accounts[4], // payer
        &accounts[5], // compressed_token_program_authority
        &accounts[6], // system_program
    );

    // Add SPL bridge config if provided
    if accounts.len() >= 10 && data.spl_interface_pda_bump.is_some() {
        transfer = transfer.with_spl_interface(SplInterfaceCpi {
            mint: &accounts[7],
            spl_token_program: &accounts[9],
            spl_interface_pda: &accounts[8],
            spl_interface_pda_bump: data.spl_interface_pda_bump.unwrap(),
        });
    }

    // Invoke with PDA signing
    let authority_bump_byte = [authority_bump];
    let authority_seeds = [
        Seed::from(TRANSFER_INTERFACE_AUTHORITY_SEED),
        Seed::from(&authority_bump_byte[..]),
    ];
    let authority_signer = Signer::from(&authority_seeds);
    transfer.invoke_signed(&[authority_signer])?;

    Ok(())
}
