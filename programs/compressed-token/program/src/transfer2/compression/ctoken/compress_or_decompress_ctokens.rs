use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_account_checks::checks::check_owner;
use light_compressed_account::Pubkey;
use light_ctoken_interface::{
    instructions::{extensions::ZExtensionInstructionData, transfer2::ZCompressionMode},
    state::{CToken, ZCTokenMut, ZExtensionStructMut},
    CTokenError,
};
use light_program_profiler::profile;
use light_zero_copy::traits::ZeroCopyAtMut;
use pinocchio::{
    account_info::AccountInfo,
    pubkey::pubkey_eq,
    sysvars::{clock::Clock, rent::Rent, Sysvar},
};
use spl_pod::solana_msg::msg;

use super::{compress_and_close::process_compress_and_close, inputs::CTokenCompressionInputs};
use crate::shared::owner_validation::check_ctoken_owner;

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
        input_tlv,
        input_delegate,
    } = inputs;

    check_owner(&crate::LIGHT_CPI_SIGNER.program_id, token_account_info)?;
    let mut token_account_data = token_account_info
        .try_borrow_mut_data()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;

    let (mut ctoken, _) = CToken::zero_copy_at_mut(&mut token_account_data)?;

    // Account type check: must be CToken account (byte 165 == 2)
    // SPL token accounts are exactly 165 bytes and don't have this field.
    // CToken accounts are longer and have account_type at byte 165.
    if !ctoken.is_ctoken_account() {
        msg!("Invalid account type");
        return Err(CTokenError::InvalidAccountType.into());
    }
    // Reject uninitialized accounts (state == 0)
    // Frozen accounts (state == 2) are allowed for CompressAndClose (checked below)
    if ctoken.base.state == 0 {
        msg!("Account is uninitialized");
        return Err(CTokenError::InvalidAccountState.into());
    }
    if !pubkey_eq(ctoken.mint.array_ref(), &mint) {
        msg!(
            "mint mismatch account: ctoken.mint {:?}, mint {:?}",
            solana_pubkey::Pubkey::new_from_array(ctoken.mint.to_bytes()),
            solana_pubkey::Pubkey::new_from_array(mint)
        );
        return Err(ProgramError::InvalidAccountData);
    }

    // Check if account is frozen (SPL Token-2022 compatibility)
    // Frozen accounts cannot have their balance modified except for CompressAndClose
    // (only foresters can call CompressAndClose via registry program)
    if ctoken.base.state == 2 && mode != ZCompressionMode::CompressAndClose {
        msg!("Cannot modify frozen account");
        return Err(ErrorCode::AccountFrozen.into());
    }
    // Get current balance
    let current_balance: u64 = ctoken.base.amount.get();
    let mut current_slot = 0;
    // Calculate new balance using effective amount
    match mode {
        ZCompressionMode::Compress => {
            // Verify authority for compression operations
            let authority_account = authority.ok_or(ErrorCode::InvalidCompressAuthority)?;
            check_ctoken_owner(&mut ctoken, authority_account, mint_checks.as_ref())?;

            // Compress: subtract from solana account
            // Update the balance in the ctoken solana account
            ctoken.base.amount.set(
                current_balance
                    .checked_sub(amount)
                    .ok_or(ProgramError::ArithmeticOverflow)?,
            );

            process_compression_top_up(
                &ctoken.base.compression,
                token_account_info,
                &mut current_slot,
                transfer_amount,
                lamports_budget,
            )
        }
        ZCompressionMode::Decompress => {
            // Decompress: add to solana account
            // Update the balance in the compressed token account
            ctoken.base.amount.set(
                current_balance
                    .checked_add(amount)
                    .ok_or(ProgramError::ArithmeticOverflow)?,
            );

            // Handle extension state transfer from input compressed account
            apply_decompress_extension_state(&mut ctoken, input_tlv, input_delegate)?;

            process_compression_top_up(
                &ctoken.base.compression,
                token_account_info,
                &mut current_slot,
                transfer_amount,
                lamports_budget,
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

/// Apply extension state from the input compressed account during decompress.
/// This transfers delegate, delegated_amount, and withheld_transfer_fee from
/// the compressed account's CompressedOnly extension to the CToken account.
#[inline(always)]
fn apply_decompress_extension_state(
    ctoken: &mut ZCTokenMut,
    input_tlv: Option<&[ZExtensionInstructionData]>,
    input_delegate: Option<&AccountInfo>,
) -> Result<(), ProgramError> {
    // Extract CompressedOnly extension data from input TLV
    let compressed_only_data = input_tlv.and_then(|tlv| {
        tlv.iter().find_map(|ext| {
            if let ZExtensionInstructionData::CompressedOnly(data) = ext {
                Some(data)
            } else {
                None
            }
        })
    });

    // If no CompressedOnly extension, nothing to transfer
    let Some(ext_data) = compressed_only_data else {
        return Ok(());
    };

    let delegated_amount: u64 = ext_data.delegated_amount.into();
    let withheld_transfer_fee: u64 = ext_data.withheld_transfer_fee.into();

    // Handle delegate and delegated_amount
    if delegated_amount > 0 || input_delegate.is_some() {
        let input_delegate_pubkey = input_delegate.map(|acc| Pubkey::from(*acc.key()));

        // Validate delegate compatibility
        if let Some(ctoken_delegate) = ctoken.delegate() {
            // CToken has a delegate - check if it matches the input delegate
            if let Some(input_del) = input_delegate_pubkey.as_ref() {
                if ctoken_delegate.to_bytes() != input_del.to_bytes() {
                    msg!(
                        "Decompress delegate mismatch: CToken delegate {:?} != input delegate {:?}",
                        ctoken_delegate.to_bytes(),
                        input_del.to_bytes()
                    );
                    return Err(ErrorCode::DecompressDelegateMismatch.into());
                }
            }
            // Delegates match - add to delegated_amount
        } else if let Some(input_del) = input_delegate_pubkey {
            // CToken has no delegate - set it from the input
            ctoken.base.set_delegate(Some(input_del))?;
        } else if delegated_amount > 0 {
            // Has delegated_amount but no delegate pubkey - invalid state
            msg!("Decompress: delegated_amount > 0 but no delegate pubkey provided");
            return Err(CTokenError::InvalidAccountData.into());
        }

        // Add delegated_amount to CToken's delegated_amount
        if delegated_amount > 0 {
            let current_delegated: u64 = ctoken.base.delegated_amount.get();
            ctoken.base.delegated_amount.set(
                current_delegated
                    .checked_add(delegated_amount)
                    .ok_or(ProgramError::ArithmeticOverflow)?,
            );
        }
    }

    // Handle withheld_transfer_fee
    if withheld_transfer_fee > 0 {
        let mut fee_applied = false;
        if let Some(extensions) = ctoken.extensions.as_deref_mut() {
            for extension in extensions.iter_mut() {
                if let ZExtensionStructMut::TransferFeeAccount(ref mut fee_ext) = extension {
                    fee_ext.add_withheld_amount(withheld_transfer_fee)?;
                    fee_applied = true;
                    break;
                }
            }
        }
        if !fee_applied {
            msg!("Decompress: withheld_transfer_fee > 0 but no TransferFeeAccount extension found");
            return Err(CTokenError::InvalidAccountData.into());
        }
    }

    // Handle is_frozen - restore frozen state from compressed token
    if ext_data.is_frozen != 0 {
        ctoken.base.set_frozen();
    }

    Ok(())
}

/// Process compression top-up using embedded compression info.
/// All ctoken accounts now have compression info embedded directly in meta.
#[inline(always)]
pub fn process_compression_top_up(
    compression: &light_compressible::compression_info::ZCompressionInfoMut<'_>,
    token_account_info: &AccountInfo,
    current_slot: &mut u64,
    transfer_amount: &mut u64,
    lamports_budget: &mut u64,
) -> Result<(), ProgramError> {
    if *transfer_amount != 0 {
        return Ok(());
    }

    if *current_slot == 0 {
        *current_slot = Clock::get()
            .map_err(|_| CTokenError::SysvarAccessError)?
            .slot;
    }
    let rent_exemption = Rent::get()
        .map_err(|_| CTokenError::SysvarAccessError)?
        .minimum_balance(token_account_info.data_len());

    *transfer_amount = compression
        .calculate_top_up_lamports(
            token_account_info.data_len() as u64,
            *current_slot,
            token_account_info.lamports(),
            rent_exemption,
        )
        .map_err(|_| CTokenError::InvalidAccountData)?;

    *lamports_budget = lamports_budget.saturating_sub(*transfer_amount);

    Ok(())
}
