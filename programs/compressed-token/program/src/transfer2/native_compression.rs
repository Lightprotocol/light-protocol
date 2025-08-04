use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_account_checks::checks::check_owner;
use light_ctoken_types::instructions::transfer2::{
    CompressionMode, ZCompressedTokenInstructionDataTransfer2, ZCompression,
};
use pinocchio::account_info::AccountInfo;
use solana_pubkey::Pubkey;
use spl_pod::{bytemuck::pod_from_bytes_mut, solana_msg::msg};
use spl_token_2022::pod::PodAccount;

use crate::{
    shared::owner_validation::verify_and_update_token_account_authority_with_pod,
    transfer2::accounts::ProgramPackedAccounts, LIGHT_CPI_SIGNER,
};
const ID: &[u8; 32] = &LIGHT_CPI_SIGNER.program_id;
/// Process native compressions/decompressions with token accounts
pub fn process_token_compression(
    inputs: &ZCompressedTokenInstructionDataTransfer2,
    packed_accounts: &ProgramPackedAccounts,
) -> Result<(), ProgramError> {
    if let Some(compressions) = inputs.compressions.as_ref() {
        for compression in compressions {
            let source_or_recipient = packed_accounts.get_u8(compression.source_or_recipient)?;

            match unsafe { source_or_recipient.owner() } {
                ID => {
                    process_native_compressions(compression, source_or_recipient, packed_accounts)?;
                }
                _ => return Err(ProgramError::InvalidInstructionData),
            }
        }
    }
    Ok(())
}

/// Validate compression fields based on compression mode
fn validate_compression_mode_fields(compression: &ZCompression) -> Result<(), ProgramError> {
    let mode = compression.mode;

    match mode {
        CompressionMode::Decompress => {
            // Decompress must have authority = 0
            if compression.authority != 0 {
                msg!("authority must be 0 for Decompress mode");
                return Err(ProgramError::InvalidInstructionData);
            }
        }
        CompressionMode::Compress => {
            // No additional validation needed for regular compress
        }
    }

    Ok(())
}

/// Process compression/decompression for token accounts using zero-copy PodAccount
fn process_native_compressions(
    compression: &ZCompression,
    token_account_info: &AccountInfo,
    packed_accounts: &ProgramPackedAccounts,
) -> Result<(), ProgramError> {
    let mode = compression.mode;

    // Validate compression fields for the given mode
    validate_compression_mode_fields(compression)?;
    // Get authority account and effective compression amount
    let authority_account = packed_accounts.get_u8(compression.authority)?;
    // TODO: add get_checked_account from  PackedAccounts.
    let mint_account = *packed_accounts.get_u8(compression.mint)?.key();
    native_compression(
        Some(&authority_account),
        (*compression.amount).into(),
        mint_account.into(),
        token_account_info,
        mode,
    )?;

    Ok(())
}

/// Perform native compression/decompression on a token account
pub fn native_compression(
    authority: Option<&AccountInfo>,
    amount: u64,
    mint: Pubkey,
    token_account_info: &AccountInfo,
    mode: CompressionMode,
) -> Result<(), ProgramError> {
    msg!(
        "token_account_info {:?}",
        solana_pubkey::Pubkey::new_from_array(*token_account_info.key())
    );
    check_owner(&crate::LIGHT_CPI_SIGNER.program_id, token_account_info)?;
    // Access token account data as mutable bytes
    let mut token_account_data = token_account_info
        .try_borrow_mut_data()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;

    // Use zero-copy PodAccount to access the token account
    let pod_account = pod_from_bytes_mut::<PodAccount>(&mut token_account_data)
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;

    if pod_account.mint != mint {
        msg!(
            "mint mismatch account: pod_account.mint {:?}, mint {:?}",
            pod_account.mint,
            solana_pubkey::Pubkey::new_from_array(mint.to_bytes())
        );
        return Err(ProgramError::InvalidAccountData);
    }

    // Get current balance
    let current_balance: u64 = pod_account.amount.into();

    // Calculate new balance using effective amount
    let new_balance = match mode {
        CompressionMode::Compress => {
            // Verify authority for compression operations and update delegated amount if needed
            let authority_account = authority.ok_or(ErrorCode::InvalidCompressAuthority)?;
            verify_and_update_token_account_authority_with_pod(
                pod_account,
                authority_account,
                amount,
            )?;

            // Compress: subtract from solana account
            current_balance
                .checked_sub(amount)
                .ok_or(ProgramError::ArithmeticOverflow)?
        }
        CompressionMode::Decompress => {
            // Decompress: add to solana account
            current_balance
                .checked_add(amount)
                .ok_or(ProgramError::ArithmeticOverflow)?
        }
    };

    // Update the balance in the pod account
    pod_account.amount = new_balance.into();

    Ok(())
}
