use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_account_checks::{checks::check_owner, packed_accounts::ProgramPackedAccounts};
use light_ctoken_types::{
    instructions::transfer2::{CompressionMode, ZCompression},
    state::CompressedToken,
};
use light_zero_copy::traits::ZeroCopyAtMut;
use pinocchio::account_info::AccountInfo;
use solana_pubkey::Pubkey;
use spl_pod::solana_msg::msg;

use super::validate_compression_mode_fields;
use crate::shared::owner_validation::verify_and_update_token_account_authority_with_compressed_token;

/// Process compression/decompression for token accounts using zero-copy PodAccount
pub(super) fn process_native_compressions(
    compression: &ZCompression,
    token_account_info: &AccountInfo,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
) -> Result<(), ProgramError> {
    let mode = compression.mode;

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
    native_compression(
        Some(authority_account),
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
    let new_balance = match mode {
        CompressionMode::Compress => {
            // Verify authority for compression operations and update delegated amount if needed
            let authority_account = authority.ok_or(ErrorCode::InvalidCompressAuthority)?;
            verify_and_update_token_account_authority_with_compressed_token(
                &mut compressed_token,
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

    // Update the balance in the compressed token account
    *compressed_token.amount = new_balance.into();

    compressed_token
        .update_compressible_last_written_slot()
        .map_err(|_| ProgramError::InvalidAccountData)?;
    Ok(())
}
