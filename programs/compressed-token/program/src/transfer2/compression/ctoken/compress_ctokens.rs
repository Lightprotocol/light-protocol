use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_account_checks::checks::check_owner;
use light_ctoken_types::{
    instructions::transfer2::ZCompressionMode,
    state::{CToken, ZExtensionStructMut},
    CTokenError,
};
use light_profiler::profile;
use light_zero_copy::traits::ZeroCopyAtMut;
use pinocchio::{
    account_info::AccountInfo,
    sysvars::{clock::Clock, Sysvar},
};
use spl_pod::solana_msg::msg;

use super::{compress_and_close::process_compress_and_close, inputs::NativeCompressionInputs};
use crate::shared::owner_validation::verify_and_update_token_account_authority_with_compressed_token;

/// Perform compression/decompression on a ctoken account
#[profile]
pub fn compress_ctokens(inputs: NativeCompressionInputs) -> Result<Option<u64>, ProgramError> {
    let NativeCompressionInputs {
        authority,
        compress_and_close_inputs,
        amount,
        mint,
        token_account_info,
        mode,
        packed_accounts,
    } = inputs;

    check_owner(&crate::LIGHT_CPI_SIGNER.program_id, token_account_info)?;
    let mut token_account_data = token_account_info
        .try_borrow_mut_data()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;

    let (mut ctoken, _) = CToken::zero_copy_at_mut(&mut token_account_data)
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if ctoken.mint.to_bytes() != mint {
        msg!(
            "mint mismatch account: ctoken.mint {:?}, mint {:?}",
            solana_pubkey::Pubkey::new_from_array(ctoken.mint.to_bytes()),
            solana_pubkey::Pubkey::new_from_array(mint)
        );
        return Err(ProgramError::InvalidAccountData);
    }

    // Get current balance
    let current_balance: u64 = u64::from(*ctoken.amount);
    let mut current_slot = 0;
    // Calculate new balance using effective amount
    match mode {
        ZCompressionMode::Compress => {
            // Verify authority for compression operations and update delegated amount if needed
            let authority_account = authority.ok_or(ErrorCode::InvalidCompressAuthority)?;
            verify_and_update_token_account_authority_with_compressed_token(
                &mut ctoken,
                authority_account,
                amount,
            )?;

            // Compress: subtract from solana account
            // Update the balance in the ctoken solana account
            *ctoken.amount = current_balance
                .checked_sub(amount)
                .ok_or(ProgramError::ArithmeticOverflow)?
                .into();

            process_compressible_extension(
                ctoken.extensions.as_deref(),
                token_account_info,
                &mut current_slot,
            )
        }
        ZCompressionMode::Decompress => {
            // Decompress: add to solana account
            // Update the balance in the compressed token account
            *ctoken.amount = current_balance
                .checked_add(amount)
                .ok_or(ProgramError::ArithmeticOverflow)?
                .into();

            process_compressible_extension(
                ctoken.extensions.as_deref(),
                token_account_info,
                &mut current_slot,
            )
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

#[inline(always)]
fn process_compressible_extension(
    extensions: Option<&[ZExtensionStructMut]>,
    token_account_info: &AccountInfo,
    current_slot: &mut u64,
) -> Result<Option<u64>, ProgramError> {
    if let Some(extensions) = extensions {
        for extension in extensions.iter() {
            if let ZExtensionStructMut::Compressible(compressible_extension) = extension {
                if *current_slot == 0 {
                    *current_slot = Clock::get()
                        .map_err(|_| CTokenError::SysvarAccessError)?
                        .slot;
                }
                let transfer_amount = compressible_extension
                    .calculate_top_up_lamports(
                        token_account_info.data_len() as u64,
                        *current_slot,
                        token_account_info.lamports(),
                        compressible_extension.lamports_per_write.into(),
                    )
                    .map_err(|_| CTokenError::InvalidAccountData)?;

                return Ok(Some(transfer_amount));
            }
        }
    }
    Ok(None)
}
