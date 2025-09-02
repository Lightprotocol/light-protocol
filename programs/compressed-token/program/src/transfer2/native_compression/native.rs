use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_account_checks::{checks::check_owner, packed_accounts::ProgramPackedAccounts};
use light_ctoken_types::{
    instructions::transfer2::{
        ZCompressedTokenInstructionDataTransfer2, ZCompression, ZCompressionMode,
        ZMultiTokenTransferOutputData,
    },
    state::{CompressedToken, ZCompressedTokenMut},
    CTokenError,
};
use light_zero_copy::traits::ZeroCopyAtMut;
use pinocchio::account_info::AccountInfo;
use solana_pubkey::Pubkey;
use spl_pod::solana_msg::msg;

use super::validate_compression_mode_fields;
use crate::{
    close_token_account::{accounts::CloseTokenAccountAccounts, processor::validate_token_account},
    shared::owner_validation::verify_and_update_token_account_authority_with_compressed_token,
};

/// Process compression/decompression for token accounts using zero-copy PodAccount
pub(super) fn process_native_compressions(
    inputs: &ZCompressedTokenInstructionDataTransfer2,
    compression: &ZCompression,
    token_account_info: &AccountInfo,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
) -> Result<(), ProgramError> {
    let mode = &compression.mode;

    // Validate compression fields for the given mode
    validate_compression_mode_fields(compression)?;
    // Get authority account and effective compression amount
    let authority_account = packed_accounts.get_u8(
        compression.authority,
        "process_native_compression: authority",
    )?;

    let mint_account = *packed_accounts
        .get_u8(compression.mint, "process_native_compression: token mint")?
        .key();
    let (destination, compressed_token_account) = if *mode == ZCompressionMode::CompressAndClose {
        let compressed_token_account = inputs
            .out_token_data
            .get(compression.get_compressed_token_account_index()? as usize)
            .ok_or(CTokenError::AccountFrozen)?;
        (
            Some(packed_accounts.get_u8(
                compression.get_rent_recipient_index()?,
                "process_native_compression: token mint",
            )?),
            Some(compressed_token_account),
        )
    } else {
        (None, None)
    };

    native_compression(
        Some(authority_account),
        compressed_token_account,
        (*compression.amount).into(),
        mint_account.into(),
        token_account_info,
        destination,
        mode,
        packed_accounts,
    )?;

    Ok(())
}

/// Perform native compression/decompression on a token account
#[allow(clippy::too_many_arguments)]
pub fn native_compression(
    authority: Option<&AccountInfo>,
    compressed_token_account: Option<&ZMultiTokenTransferOutputData<'_>>,
    amount: u64,
    mint: Pubkey,
    token_account_info: &AccountInfo,
    destination: Option<&AccountInfo>,
    mode: &ZCompressionMode,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
) -> Result<(), ProgramError> {
    check_owner(&crate::LIGHT_CPI_SIGNER.program_id, token_account_info)?;
    let mut token_account_data = token_account_info
        .try_borrow_mut_data()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;

    let (mut compressed_token, _) = CompressedToken::zero_copy_at_mut(&mut token_account_data)
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if compressed_token.mint.to_bytes() != mint.to_bytes() {
        msg!(
            "mint mismatch account: compressed_token.mint {:?}, mint {:?}",
            solana_pubkey::Pubkey::new_from_array(compressed_token.mint.to_bytes()),
            solana_pubkey::Pubkey::new_from_array(mint.to_bytes())
        );
        return Err(ProgramError::InvalidAccountData);
    }

    // Get current balance
    let current_balance: u64 = u64::from(*compressed_token.amount);
    // Calculate new balance using effective amount
    match mode {
        ZCompressionMode::Compress => {
            // Verify authority for compression operations and update delegated amount if needed
            let authority_account = authority.ok_or(ErrorCode::InvalidCompressAuthority)?;
            verify_and_update_token_account_authority_with_compressed_token(
                &mut compressed_token,
                authority_account,
                amount,
            )?;

            // Compress: subtract from solana account
            let new_balance = current_balance
                .checked_sub(amount)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            // Update the balance in the compressed token account
            *compressed_token.amount = new_balance.into();

            compressed_token
                .update_compressible_last_written_slot()
                .map_err(|_| ProgramError::InvalidAccountData)?;
        }
        ZCompressionMode::Decompress => {
            // Decompress: add to solana account
            let new_balance = current_balance
                .checked_add(amount)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            // Update the balance in the compressed token account
            *compressed_token.amount = new_balance.into();

            compressed_token
                .update_compressible_last_written_slot()
                .map_err(|_| ProgramError::InvalidAccountData)?;
        }
        ZCompressionMode::CompressAndClose => {
            {
                // Compress the complete balance to this compressed token account.
                validate_compressed_token_account(
                    packed_accounts,
                    amount,
                    compressed_token_account.ok_or(CTokenError::InvalidCompressionMode)?,
                    &compressed_token,
                )?;
                *compressed_token.amount = 0.into();
            }
            validate_token_account(
                &CloseTokenAccountAccounts {
                    token_account: token_account_info,
                    destination: destination
                        .ok_or(ErrorCode::CompressAndCloseDestinationMissing)?,
                    authority: authority.ok_or(ErrorCode::CompressAndCloseAuthorityMissing)?,
                },
                &compressed_token,
            )?;
            return Ok(());
        }
    };

    Ok(())
}

fn validate_compressed_token_account(
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
    compression_amount: u64,
    compressed_token_account: &ZMultiTokenTransferOutputData<'_>,
    compressed_token: &ZCompressedTokenMut,
) -> Result<(), ProgramError> {
    // Owner should match
    if *compressed_token.owner
        != *packed_accounts
            .get_u8(compressed_token_account.owner, "CompressAndClose: owner")?
            .key()
    {
        return Err(ErrorCode::CompressAndCloseInvalidOwner.into());
    }
    // Compression amount must match the output amount
    if compression_amount != compressed_token_account.amount.get() {
        msg!(
            "compression_amount {} != compressed token account amount {}",
            compression_amount,
            compressed_token_account.amount.get()
        );
        return Err(ErrorCode::CompressAndCloseAmountMismatch.into());
    }
    // Token balance must match the compressed output amount
    if *compressed_token.amount != compressed_token_account.amount {
        msg!(
            "output compressed_token.amount {} != compressed token account amount {}",
            compressed_token.amount,
            compressed_token_account.amount.get()
        );
        return Err(ErrorCode::CompressAndCloseBalanceMismatch.into());
    }
    // Delegate should be None
    if compressed_token_account.has_delegate() {
        return Err(ErrorCode::CompressAndCloseDelegateNotAllowed.into());
    }
    if compressed_token_account.delegate != 0 {
        return Err(ErrorCode::CompressAndCloseDelegateNotAllowed.into());
    }
    // Version should be 2
    if compressed_token_account.version != 2 {
        return Err(ErrorCode::CompressAndCloseInvalidVersion.into());
    }
    Ok(())
}
