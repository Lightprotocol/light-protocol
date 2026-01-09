use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use bitvec::prelude::*;
use light_account_checks::{checks::check_signer, packed_accounts::ProgramPackedAccounts};
use light_ctoken_interface::{
    instructions::{
        extensions::ZExtensionInstructionData,
        transfer2::{ZCompression, ZCompressionMode, ZMultiTokenTransferOutputData},
    },
    state::{TokenDataVersion, ZCTokenMut, ZExtensionStructMut},
    CTokenError,
};
use light_program_profiler::profile;
use pinocchio::{
    account_info::AccountInfo,
    pubkey::{pubkey_eq, Pubkey},
};
use spl_pod::solana_msg::msg;

use super::inputs::CompressAndCloseInputs;
use crate::{
    compressed_token::transfer2::accounts::Transfer2Accounts,
    ctoken::close::{
        accounts::CloseTokenAccountAccounts, processor::validate_token_account_for_close_transfer2,
    },
};

/// Process compress and close operation for a ctoken account.
#[profile]
pub fn process_compress_and_close(
    authority: Option<&AccountInfo>,
    compress_and_close_inputs: Option<CompressAndCloseInputs>,
    amount: u64,
    token_account_info: &AccountInfo,
    ctoken: &mut ZCTokenMut,
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
    let compress_to_pubkey = validate_token_account_for_close_transfer2(
        &CloseTokenAccountAccounts {
            token_account: token_account_info,
            destination: close_inputs.destination,
            authority,
            rent_sponsor: Some(close_inputs.rent_sponsor),
        },
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
        compress_to_pubkey,
        token_account_info.key(),
        close_inputs.tlv,
    )?;
    // TODO: remove once we separated close logic for compress and close
    //TODO: introduce ready to close state and set it here
    {
        ctoken.base.amount.set(0);
        // Unfreeze the account if frozen (frozen state is preserved in compressed token TLV)
        // This allows the close_token_account validation to pass for frozen accounts
        ctoken.base.set_initialized();
    }
    Ok(())
}

/// Validate compressed token account for compress and close operation
fn validate_compressed_token_account(
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
    compression_amount: u64,
    compressed_token_account: &ZMultiTokenTransferOutputData<'_>,
    ctoken: &ZCTokenMut,
    compress_to_pubkey: bool,
    token_account_pubkey: &Pubkey,
    out_tlv: Option<&[ZExtensionInstructionData<'_>]>,
) -> Result<(), ProgramError> {
    let compression = ctoken
        .get_compressible_extension()
        .ok_or::<ProgramError>(CTokenError::MissingCompressibleExtension.into())?;
    let is_ata = compression.is_ata != 0;
    // Owners should match if not compressing to pubkey
    if compress_to_pubkey || is_ata {
        // what about is ata?
        // Owner should match token account pubkey if compressing to pubkey
        if *packed_accounts
            .get_u8(compressed_token_account.owner, "CompressAndClose: owner")?
            .key()
            != *token_account_pubkey
        {
            msg!(
                "compress_to_pubkey: packed_accounts owner {:?} should match token_account_pubkey: {:?}",
                solana_pubkey::Pubkey::new_from_array(
                    *packed_accounts
                        .get_u8(compressed_token_account.owner, "CompressAndClose: owner")?
                        .key()
                ),
                solana_pubkey::Pubkey::new_from_array(*token_account_pubkey)
            );
            return Err(ErrorCode::CompressAndCloseInvalidOwner.into());
        }
    } else if ctoken.owner.to_bytes()
        != *packed_accounts
            .get_u8(compressed_token_account.owner, "CompressAndClose: owner")?
            .key()
    {
        msg!(
            "*ctoken.owner {:?} packed_accounts owner: {:?}",
            solana_pubkey::Pubkey::new_from_array(ctoken.owner.to_bytes()),
            solana_pubkey::Pubkey::new_from_array(
                *packed_accounts
                    .get_u8(compressed_token_account.owner, "CompressAndClose: owner")?
                    .key()
            )
        );
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
    if ctoken.amount.get() != compressed_token_account.amount.get() {
        msg!(
            "output ctoken.amount {} != compressed token account amount {}",
            ctoken.amount.get(),
            compressed_token_account.amount.get()
        );
        return Err(ErrorCode::CompressAndCloseBalanceMismatch.into());
    }

    // Mint must match
    let output_mint = packed_accounts
        .get_u8(compressed_token_account.mint, "CompressAndClose: mint")?
        .key();
    if *output_mint != ctoken.mint.to_bytes() {
        msg!(
            "mint mismatch: ctoken {:?} != output {:?}",
            solana_pubkey::Pubkey::new_from_array(ctoken.mint.to_bytes()),
            solana_pubkey::Pubkey::new_from_array(*output_mint)
        );
        return Err(ErrorCode::CompressAndCloseInvalidMint.into());
    }

    // Version should be ShaFlat
    if compressed_token_account.version != TokenDataVersion::ShaFlat as u8 {
        return Err(ErrorCode::CompressAndCloseInvalidVersion.into());
    }

    // Version should also match what's specified in the embedded compression info
    let expected_version = compression.info.account_version;
    let compression_only = compression.compression_only();

    if compressed_token_account.version != expected_version {
        return Err(ErrorCode::CompressAndCloseInvalidVersion.into());
    }
    let compression_only_extension = out_tlv.as_ref().and_then(|ext| {
        ext.iter()
            .find(|e| matches!(e, ZExtensionInstructionData::CompressedOnly(_)))
    });

    // CompressedOnly extension is required for:
    // - compression_only accounts (cannot decompress to SPL)
    // - ATA accounts (need is_ata flag for proper decompress authorization)
    if (compression_only || is_ata) && compression_only_extension.is_none() {
        return Err(ErrorCode::CompressAndCloseMissingCompressedOnlyExtension.into());
    }

    if let Some(ZExtensionInstructionData::CompressedOnly(compression_only_extension)) =
        compression_only_extension
    {
        // Note: is_ata validation happens during decompress, not compress_and_close.
        // During compress_and_close we just store the is_ata flag from the Compressible extension.
        // The decompress instruction validates the ATA derivation using the stored is_ata and bump.

        // Delegated amounts must match
        if u64::from(compression_only_extension.delegated_amount) != ctoken.delegated_amount.get() {
            msg!(
                "delegated_amount mismatch: ctoken {} != extension {}",
                ctoken.delegated_amount.get(),
                u64::from(compression_only_extension.delegated_amount)
            );
            return Err(ErrorCode::CompressAndCloseDelegatedAmountMismatch.into());
        }
        // Delegate must be preserved for exact state restoration during decompress
        if ctoken.delegate().is_some() || compression_only_extension.delegated_amount != 0 {
            let delegate = ctoken
                .delegate()
                .ok_or(ErrorCode::CompressAndCloseInvalidDelegate)?;
            if !compressed_token_account.has_delegate() {
                msg!("ctoken has delegate but compressed token output does not");
                return Err(ErrorCode::CompressAndCloseInvalidDelegate.into());
            }
            let token_data_delegate = packed_accounts.get_u8(
                compressed_token_account.delegate,
                "compressed_token_account delegate",
            )?;
            if !pubkey_eq(token_data_delegate.key(), &delegate.to_bytes()) {
                msg!(
                    "delegate mismatch: ctoken {:?} != output {:?}",
                    solana_pubkey::Pubkey::new_from_array(delegate.to_bytes()),
                    solana_pubkey::Pubkey::new_from_array(*token_data_delegate.key())
                );
                return Err(ErrorCode::CompressAndCloseInvalidDelegate.into());
            }
        }
        // if ctoken has fee extension withheld amount must match
        let ctoken_withheld_fee = ctoken.extensions.as_ref().and_then(|exts| {
            exts.iter().find_map(|ext| {
                if let ZExtensionStructMut::TransferFeeAccount(fee_ext) = ext {
                    Some(fee_ext.withheld_amount)
                } else {
                    None
                }
            })
        });

        if let Some(withheld_fee) = ctoken_withheld_fee {
            if compression_only_extension.withheld_transfer_fee != withheld_fee {
                msg!(
                    "withheld_transfer_fee mismatch: ctoken {} != extension {}",
                    withheld_fee,
                    u64::from(compression_only_extension.withheld_transfer_fee)
                );
                return Err(ErrorCode::CompressAndCloseWithheldFeeMismatch.into());
            }
        } else if u64::from(compression_only_extension.withheld_transfer_fee) != 0 {
            msg!(
                "withheld_transfer_fee must be 0 when ctoken has no fee extension, got {}",
                u64::from(compression_only_extension.withheld_transfer_fee)
            );
            return Err(ErrorCode::CompressAndCloseWithheldFeeMismatch.into());
        }

        // Frozen state must match between CToken and extension data
        if ctoken.state != compression_only_extension.is_frozen {
            msg!(
                "is_frozen mismatch: ctoken {} != extension {}",
                ctoken.state,
                compression_only_extension.is_frozen
            );
            return Err(ErrorCode::CompressAndCloseFrozenMismatch.into());
        }
    } else {
        // Frozen accounts require CompressedOnly extension to preserve frozen state
        // AccountState::Frozen = 2 in CToken
        let ctoken_is_frozen = ctoken.state == 2;
        if ctoken_is_frozen {
            msg!("Frozen account requires CompressedOnly extension with is_frozen=true");
            return Err(ErrorCode::CompressAndCloseMissingCompressedOnlyExtension.into());
        }

        // Source token account must not have a delegate
        // Compressed tokens don't support delegation, so we reject accounts with delegates
        if ctoken.delegate().is_some() {
            msg!("Source token account has delegate, cannot compress and close");
            return Err(ErrorCode::CompressAndCloseDelegateNotAllowed.into());
        }

        // Delegate should be None
        if compressed_token_account.has_delegate() {
            return Err(ErrorCode::CompressAndCloseDelegateNotAllowed.into());
        }
        if compressed_token_account.delegate != 0 {
            return Err(ErrorCode::CompressAndCloseDelegateNotAllowed.into());
        }
    }

    Ok(())
}

/// Close ctoken accounts after compress and close operations
pub fn close_for_compress_and_close(
    compressions: &[ZCompression<'_>],
    _validated_accounts: &Transfer2Accounts,
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
            let validated_accounts = _validated_accounts;
            let token_account_info = validated_accounts.packed_accounts.get_u8(
                compression.source_or_recipient,
                "CompressAndClose: source_or_recipient",
            )?;
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
