use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_token_sdk::ctoken::TransferInterface;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::ID;

/// PDA seed for authority in invoke_signed variants
pub const TRANSFER_INTERFACE_AUTHORITY_SEED: &[u8] = b"transfer_interface_authority";

/// Instruction data for TransferInterface
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TransferInterfaceData {
    pub amount: u64,
    /// Required for SPL<->CToken transfers, None for CToken->CToken
    pub token_pool_pda_bump: Option<u8>,
}

/// Handler for TransferInterface (invoke)
///
/// This unified interface automatically detects account types and routes to:
/// - CToken -> CToken transfer
/// - CToken -> SPL transfer
/// - SPL -> CToken transfer
///
/// Account order:
/// - accounts[0]: compressed_token_program (for CPI)
/// - accounts[1]: source_account (SPL or CToken)
/// - accounts[2]: destination_account (SPL or CToken)
/// - accounts[3]: authority (signer)
/// - accounts[4]: payer (signer)
/// - accounts[5]: compressed_token_program_authority
///   For SPL bridge (optional, required for SPL<->CToken):
/// - accounts[6]: mint
/// - accounts[7]: token_pool_pda
/// - accounts[8]: spl_token_program
pub fn process_transfer_interface_invoke(
    accounts: &[AccountInfo],
    data: TransferInterfaceData,
) -> Result<(), ProgramError> {
    if accounts.len() < 6 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let mut transfer = TransferInterface::new(
        data.amount,
        accounts[1].clone(), // source_account
        accounts[2].clone(), // destination_account
        accounts[3].clone(), // authority
        accounts[4].clone(), // payer
        accounts[5].clone(), // compressed_token_program_authority
    );

    // Add SPL bridge config if provided
    if accounts.len() >= 9 && data.token_pool_pda_bump.is_some() {
        transfer = transfer.with_spl_interface(
            Some(accounts[6].clone()), // mint
            Some(accounts[8].clone()), // spl_token_program
            Some(accounts[7].clone()), // token_pool_pda
            data.token_pool_pda_bump,
        )?;
    }

    transfer.invoke()?;

    Ok(())
}

/// Handler for TransferInterface with PDA authority (invoke_signed)
///
/// The authority is a PDA derived from TRANSFER_INTERFACE_AUTHORITY_SEED.
///
/// Account order:
/// - accounts[0]: compressed_token_program (for CPI)
/// - accounts[1]: source_account (SPL or CToken)
/// - accounts[2]: destination_account (SPL or CToken)
/// - accounts[3]: authority (PDA, not signer - program signs)
/// - accounts[4]: payer (signer)
/// - accounts[5]: compressed_token_program_authority
///   For SPL bridge (optional, required for SPL<->CToken):
/// - accounts[6]: mint
/// - accounts[7]: token_pool_pda
/// - accounts[8]: spl_token_program
pub fn process_transfer_interface_invoke_signed(
    accounts: &[AccountInfo],
    data: TransferInterfaceData,
) -> Result<(), ProgramError> {
    if accounts.len() < 6 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the authority
    let (authority_pda, authority_bump) =
        Pubkey::find_program_address(&[TRANSFER_INTERFACE_AUTHORITY_SEED], &ID);

    // Verify the authority account is the PDA we expect
    if &authority_pda != accounts[3].key {
        return Err(ProgramError::InvalidSeeds);
    }

    let mut transfer = TransferInterface::new(
        data.amount,
        accounts[1].clone(), // source_account
        accounts[2].clone(), // destination_account
        accounts[3].clone(), // authority (PDA)
        accounts[4].clone(), // payer
        accounts[5].clone(), // compressed_token_program_authority
    );

    // Add SPL bridge config if provided
    if accounts.len() >= 9 && data.token_pool_pda_bump.is_some() {
        transfer = transfer.with_spl_interface(
            Some(accounts[6].clone()), // mint
            Some(accounts[8].clone()), // spl_token_program
            Some(accounts[7].clone()), // token_pool_pda
            data.token_pool_pda_bump,
        )?;
    }

    // Invoke with PDA signing
    let authority_seeds: &[&[u8]] = &[TRANSFER_INTERFACE_AUTHORITY_SEED, &[authority_bump]];
    transfer.invoke_signed(&[authority_seeds])?;

    Ok(())
}
