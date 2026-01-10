use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_account_checks::checks::check_owner;
use light_ctoken_interface::{
    instructions::transfer2::ZCompressionMode,
    state::{CToken, ZCTokenMut},
    CTokenError,
};
use light_program_profiler::profile;
use light_zero_copy::traits::ZeroCopyAtMut;
use pinocchio::pubkey::pubkey_eq;
use spl_pod::solana_msg::msg;

use super::{
    compress_and_close::process_compress_and_close, decompress::apply_decompress_extension_state,
    inputs::CTokenCompressionInputs,
};
use crate::shared::{
    compressible_top_up::process_compression_top_up, owner_validation::check_ctoken_owner,
};

/// Perform compression/decompression on a ctoken account.
///
/// # Arguments
/// * `lamports_budget` - Mutable budget to decrement when transfer amounts are calculated.
#[profile]
pub fn compress_or_decompress_ctokens(
    inputs: CTokenCompressionInputs,
    transfer_amount: &mut u64,
    lamports_budget: &mut u64,
) -> Result<(), ProgramError> {
    let CTokenCompressionInputs {
        authority,
        compress_and_close_inputs,
        amount,
        mint, // from compression, is used in sumcheck to associate the amount with the correct mint.
        token_account_info,
        mode,
        packed_accounts,
        mint_checks,
        decompress_inputs,
    } = inputs;

    check_owner(&crate::LIGHT_CPI_SIGNER.program_id, token_account_info)?;
    let mut token_account_data = token_account_info
        .try_borrow_mut_data()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;

    let (mut ctoken, _) = CToken::zero_copy_at_mut(&mut token_account_data)?;
    validate_ctoken(&ctoken, &mint, &mode)?;

    // Get current balance
    let current_balance: u64 = ctoken.base.amount.get();
    let mut current_slot = 0;
    // Calculate new balance using effective amount
    match mode {
        ZCompressionMode::Compress => {
            // Verify authority for compression operations
            let authority_account = authority.ok_or(ErrorCode::InvalidCompressAuthority)?;
            check_ctoken_owner(&mut ctoken, authority_account, mint_checks.as_ref())?;
            if !ctoken.is_initialized() {
                return Err(CTokenError::InvalidAccountState.into());
            }

            // Compress: subtract from solana account
            // Update the balance in the ctoken solana account
            ctoken.base.amount.set(
                current_balance
                    .checked_sub(amount)
                    .ok_or(ProgramError::ArithmeticOverflow)?,
            );
            if let Some(compression) = ctoken.get_compressible_extension() {
                process_compression_top_up(
                    &compression.info,
                    token_account_info,
                    &mut current_slot,
                    transfer_amount,
                    lamports_budget,
                )?;
            }
            Ok(())
        }
        ZCompressionMode::Decompress => {
            apply_decompress_extension_state(
                token_account_info,
                &mut ctoken,
                decompress_inputs,
                packed_accounts,
                amount,
            )?;

            // Decompress: add to CToken account
            // Update the balance in the CToken solana account
            ctoken.base.amount.set(
                current_balance
                    .checked_add(amount)
                    .ok_or(ProgramError::ArithmeticOverflow)?,
            );

            if let Some(compression) = ctoken.get_compressible_extension() {
                process_compression_top_up(
                    &compression.info,
                    token_account_info,
                    &mut current_slot,
                    transfer_amount,
                    lamports_budget,
                )?;
            }
            Ok(())
        }
        ZCompressionMode::CompressAndClose => process_compress_and_close(
            authority,
            compress_and_close_inputs,
            amount,
            token_account_info,
            &mut ctoken,
            packed_accounts,
        ),
    }
}

/// Validate a CToken account for compression/decompression operations.
///
/// Checks:
/// - Account type is CToken (not SPL token)
/// - Account is initialized
/// - Account is not frozen (unless CompressAndClose mode)
/// - Mint matches expected mint
#[inline(always)]
fn validate_ctoken(
    ctoken: &ZCTokenMut,
    mint: &[u8; 32],
    mode: &ZCompressionMode,
) -> Result<(), ProgramError> {
    // Account type check: must be CToken account (byte 165 == 2)
    if !ctoken.is_ctoken_account() {
        msg!("Invalid account type");
        return Err(CTokenError::InvalidAccountType.into());
    }

    // Reject uninitialized accounts (state == 0)
    if ctoken.base.is_uninitialized() {
        msg!("Account is uninitialized");
        return Err(CTokenError::InvalidAccountState.into());
    }

    // Frozen accounts can only be modified via CompressAndClose
    if ctoken.is_frozen() && !mode.is_compress_and_close() {
        msg!("Cannot modify frozen account");
        return Err(ErrorCode::AccountFrozen.into());
    }

    // Validate mint matches
    if !pubkey_eq(ctoken.mint.array_ref(), mint) {
        msg!(
            "mint mismatch: ctoken.mint {:?}, expected {:?}",
            solana_pubkey::Pubkey::new_from_array(ctoken.mint.to_bytes()),
            solana_pubkey::Pubkey::new_from_array(*mint)
        );
        return Err(CTokenError::MintMismatch.into());
    }

    Ok(())
}
