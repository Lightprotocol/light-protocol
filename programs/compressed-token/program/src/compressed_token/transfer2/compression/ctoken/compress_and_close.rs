use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use bitvec::prelude::*;
#[cfg(target_os = "solana")]
use light_account_checks::AccountInfoTrait;
use light_account_checks::{checks::check_signer, packed_accounts::ProgramPackedAccounts};
use light_program_profiler::profile;
#[cfg(target_os = "solana")]
use light_token_interface::state::Token;
use light_token_interface::{
    instructions::{
        extensions::ZExtensionInstructionData,
        transfer2::{ZCompression, ZCompressionMode, ZMultiTokenTransferOutputData},
    },
    state::{TokenDataVersion, ZExtensionStructMut, ZTokenMut},
    TokenError,
};
use pinocchio::{
    account_info::AccountInfo,
    pubkey::{pubkey_eq, Pubkey},
    sysvars::Sysvar,
};
use spl_pod::solana_msg::msg;

use super::inputs::CompressAndCloseInputs;
#[cfg(target_os = "solana")]
use crate::ctoken::close::accounts::CloseTokenAccountAccounts;
use crate::{
    compressed_token::transfer2::accounts::Transfer2Accounts, shared::convert_program_error,
};

/// Process compress and close operation for a ctoken account.
#[profile]
pub fn process_compress_and_close(
    authority: Option<&AccountInfo>,
    compress_and_close_inputs: Option<CompressAndCloseInputs>,
    amount: u64,
    token_account_info: &AccountInfo,
    ctoken: &mut ZTokenMut,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
) -> Result<(), ProgramError> {
    let authority = authority.ok_or(ErrorCode::CompressAndCloseAuthorityMissing)?;
    check_signer(authority).map_err(|e| {
        anchor_lang::solana_program::msg!("Authority signer check failed: {:?}", e);
        ProgramError::from(e)
    })?;

    let close_inputs =
        compress_and_close_inputs.ok_or(ErrorCode::CompressAndCloseDestinationMissing)?;

    // Validate token account - only compressible accounts with compression_authority are allowed
    validate_ctoken_account(
        token_account_info,
        authority,
        close_inputs.rent_sponsor,
        ctoken,
    )?;

    // Validate compressed output matches the account being closed
    let compressed_account = close_inputs
        .compressed_token_account
        .ok_or(ErrorCode::CompressAndCloseOutputMissing)?;
    validate_compressed_token_account(
        packed_accounts,
        amount,
        compressed_account,
        ctoken,
        token_account_info.key(),
        close_inputs.tlv,
    )?;

    ctoken.base.amount.set(0);
    // Unfreeze the account if frozen (frozen state is preserved in compressed token TLV)
    // This allows the close_token_account validation to pass for frozen accounts
    ctoken.base.set_initialized();

    Ok(())
}

/// Validate compressed token account for compress and close operation.
///
/// Validations:
/// 1. Owner - output owner matches ctoken owner (or token account pubkey for ATA/compress_to_pubkey)
/// 2. Amount - compression_amount == output_amount == ctoken.amount
/// 3. Mint - output mint matches ctoken mint
/// 4. Version - must be ShaFlat
/// 5. CompressedOnly extension: required when the account has on-chain-only state
///    (compression_only, ATA, frozen, delegate, or marker extensions).
///    When not required, the extension must not be provided.
/// 6. With extension (via `validate_compressed_only_ext`):
///    6a. Delegated amount must match
///    6b. Delegate pubkey must match (if present)
///    6c. Withheld fee must match
///    6d. Frozen state must match
///    6e. is_ata must match
fn validate_compressed_token_account(
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
    compression_amount: u64,
    compressed_token_account: &ZMultiTokenTransferOutputData<'_>,
    ctoken: &ZTokenMut,
    token_account_pubkey: &Pubkey,
    out_tlv: Option<&[ZExtensionInstructionData<'_>]>,
) -> Result<(), ProgramError> {
    let compression = ctoken
        .get_compressible_extension()
        .ok_or::<ProgramError>(TokenError::MissingCompressibleExtension.into())?;

    // 1. Owner validation
    // compress_to_pubkey is derived from the extension (already fetched above)
    let output_owner = packed_accounts
        .get_u8(compressed_token_account.owner, "owner")?
        .key();
    let expected_owner = if compression.info.compress_to_pubkey() || compression.is_ata() {
        token_account_pubkey
    } else {
        &ctoken.owner.to_bytes()
    };
    if output_owner != expected_owner {
        return Err(ErrorCode::CompressAndCloseInvalidOwner.into());
    }

    // 2. Amount validation
    let output_amount = compressed_token_account.amount.get();
    if compression_amount != output_amount || ctoken.amount.get() != output_amount {
        return Err(ErrorCode::CompressAndCloseAmountMismatch.into());
    }

    // 3. Mint validation
    let output_mint = packed_accounts
        .get_u8(compressed_token_account.mint, "mint")?
        .key();
    if *output_mint != ctoken.mint.to_bytes() {
        return Err(ErrorCode::CompressAndCloseInvalidMint.into());
    }

    // 4. Version validation
    if compressed_token_account.version != TokenDataVersion::ShaFlat as u8 {
        return Err(ErrorCode::CompressAndCloseInvalidVersion.into());
    }

    // 5. CompressedOnly extension: required when the account has state that only
    //    exists on-chain (compression_only, ATA, frozen, delegate, or marker extensions).
    //    When not required, the extension must not be provided.
    let compression_only_ext = out_tlv.and_then(|tlv| {
        tlv.iter().find_map(|e| match e {
            ZExtensionInstructionData::CompressedOnly(ext) => Some(ext),
            _ => None,
        })
    });
    let needs_compressed_only = compression.compression_only()
        || compression.is_ata()
        || ctoken.is_frozen()
        || ctoken.delegate().is_some()
        || ctoken.extensions.as_ref().is_some_and(|exts| {
            exts.iter().any(|e| {
                matches!(
                    e,
                    ZExtensionStructMut::PausableAccount(_)
                        | ZExtensionStructMut::PermanentDelegateAccount(_)
                        | ZExtensionStructMut::TransferHookAccount(_)
                        | ZExtensionStructMut::TransferFeeAccount(_)
                )
            })
        });
    if needs_compressed_only {
        let ext = compression_only_ext.ok_or::<ProgramError>(
            ErrorCode::CompressAndCloseMissingCompressedOnlyExtension.into(),
        )?;
        // 6. With extension: validate delegate, withheld_fee, frozen, is_ata
        validate_compressed_only_ext(
            packed_accounts,
            compressed_token_account,
            ctoken,
            ext,
            compression,
        )
    } else if compression_only_ext.is_some() {
        Err(ProgramError::InvalidInstructionData)
    } else {
        if compressed_token_account.has_delegate() {
            return Err(ErrorCode::CompressAndCloseDelegateNotAllowed.into());
        }
        Ok(())
    }
}

/// Validate CompressedOnly extension fields match ctoken state.
/// Called from validation 6 in `validate_compressed_token_account`.
fn validate_compressed_only_ext(
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
    compressed_token_account: &ZMultiTokenTransferOutputData<'_>,
    ctoken: &ZTokenMut,
    ext: &light_token_interface::instructions::extensions::compressed_only::ZCompressedOnlyExtensionInstructionData,
    compression: &light_token_interface::state::ZCompressibleExtensionMut<'_>,
) -> Result<(), ProgramError> {
    // 6a. Delegated amount must match
    let ext_delegated: u64 = ext.delegated_amount.into();
    if ext_delegated != ctoken.delegated_amount.get() {
        return Err(ErrorCode::CompressAndCloseDelegatedAmountMismatch.into());
    }

    // 6b. Delegate pubkey must match (bidirectional check)
    if let Some(delegate) = ctoken.delegate() {
        // CToken has delegate - output must have matching delegate
        if !compressed_token_account.has_delegate() {
            return Err(ErrorCode::CompressAndCloseInvalidDelegate.into());
        }
        let output_delegate = packed_accounts
            .get_u8(compressed_token_account.delegate, "delegate")?
            .key();
        if !pubkey_eq(output_delegate, &delegate.to_bytes()) {
            return Err(ErrorCode::CompressAndCloseInvalidDelegate.into());
        }
    } else if compressed_token_account.has_delegate() {
        // CToken has no delegate - output must not have delegate
        return Err(ErrorCode::CompressAndCloseInvalidDelegate.into());
    }

    // 6c. Withheld fee must match
    let ctoken_fee = ctoken
        .extensions
        .as_ref()
        .and_then(|exts| {
            exts.iter().find_map(|e| match e {
                ZExtensionStructMut::TransferFeeAccount(f) => Some(f.withheld_amount.get()),
                _ => None,
            })
        })
        .unwrap_or(0);
    if u64::from(ext.withheld_transfer_fee) != ctoken_fee {
        return Err(ErrorCode::CompressAndCloseWithheldFeeMismatch.into());
    }

    // 6d. Frozen state must match
    if ctoken.is_frozen() != ext.is_frozen() {
        return Err(ErrorCode::CompressAndCloseFrozenMismatch.into());
    }

    // 6e. is_ata must match
    if compression.is_ata() != ext.is_ata() {
        return Err(ErrorCode::CompressAndCloseIsAtaMismatch.into());
    }

    Ok(())
}

/// Close ctoken accounts after compress and close operations
#[allow(unused_variables)]
pub fn close_for_compress_and_close(
    compressions: &[ZCompression<'_>],
    validated_accounts: &Transfer2Accounts,
) -> Result<(), ProgramError> {
    // Track used compressed account indices for CompressAndClose to prevent duplicate outputs
    let mut used_compressed_account_indices = [0u8; 32]; // 256 bits
    let used_bits = used_compressed_account_indices.view_bits_mut::<Msb0>();

    for compression in compressions
        .iter()
        .filter(|c| c.mode == ZCompressionMode::CompressAndClose)
    {
        // Check for duplicate compressed account indices in CompressAndClose operations
        let compressed_idx = compression.get_compressed_token_account_index()?;
        if let Some(mut bit) = used_bits.get_mut(compressed_idx as usize) {
            if *bit {
                msg!(
                    "Duplicate compressed account index {} in CompressAndClose operations",
                    compressed_idx
                );
                return Err(ErrorCode::CompressAndCloseDuplicateOutput.into());
            }
            *bit = true;
        } else {
            msg!("Compressed account index {} out of bounds", compressed_idx);
            return Err(ProgramError::InvalidInstructionData);
        }

        #[cfg(target_os = "solana")]
        {
            let token_account_info = validated_accounts.packed_accounts.get_u8(
                compression.source_or_recipient,
                "CompressAndClose: source_or_recipient",
            )?;

            // Verify balance is still zero before closing.
            // This catches cases where Decompress added tokens after CompressAndClose zeroed it.
            {
                let data = AccountInfoTrait::try_borrow_data(token_account_info)
                    .map_err(|_| ProgramError::AccountBorrowFailed)?;
                let amount = Token::amount_from_slice(&data)
                    .map_err(|_| ProgramError::InvalidAccountData)?;
                if amount != 0 {
                    msg!(
                        "CompressAndClose: account has non-zero balance {} at close time (decompress to closing account?)",
                        amount
                    );
                    return Err(ErrorCode::NonNativeHasBalance.into());
                }
            }

            let destination = validated_accounts.packed_accounts.get_u8(
                compression.get_destination_index()?,
                "CompressAndClose: destination",
            )?;
            let rent_sponsor = validated_accounts.packed_accounts.get_u8(
                compression.get_rent_sponsor_index()?,
                "CompressAndClose: rent_sponsor",
            )?;
            let authority = validated_accounts
                .packed_accounts
                .get_u8(compression.authority, "CompressAndClose: authority")?;
            use crate::ctoken::close::processor::close_token_account;
            close_token_account(&CloseTokenAccountAccounts {
                token_account: token_account_info,
                destination,
                authority,
                rent_sponsor: Some(rent_sponsor),
            })?;
        }
    }
    Ok(())
}

/// Validates that a ctoken solana account is ready to be compressed and closed.
/// Only the compression_authority can compress the account.
#[profile]
fn validate_ctoken_account(
    token_account: &AccountInfo,
    authority: &AccountInfo,
    rent_sponsor: &AccountInfo,
    ctoken: &ZTokenMut<'_>,
) -> Result<(), ProgramError> {
    // Check for Compressible extension
    let compressible = ctoken.get_compressible_extension();

    // CompressAndClose requires Compressible extension
    let compression = compressible.ok_or_else(|| {
        msg!("compress and close requires compressible extension");
        ProgramError::InvalidAccountData
    })?;

    // Validate rent_sponsor matches
    if compression.info.rent_sponsor != *rent_sponsor.key() {
        msg!("rent recipient mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    if compression.info.compression_authority != *authority.key() {
        msg!("compress and close requires compression authority");
        return Err(ProgramError::InvalidAccountData);
    }

    let current_slot = pinocchio::sysvars::clock::Clock::get()
        .map_err(convert_program_error)?
        .slot;
    compression
        .info
        .is_compressible(
            token_account.data_len() as u64,
            current_slot,
            token_account.lamports(),
        )
        .map_err(|_| ProgramError::InvalidAccountData)?
        .ok_or_else(|| {
            msg!("account not compressible");
            ProgramError::InvalidAccountData
        })?;

    Ok(())
}
