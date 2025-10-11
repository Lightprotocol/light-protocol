use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_account_checks::{checks::check_signer, packed_accounts::ProgramPackedAccounts};
use light_ctoken_types::{
    instructions::transfer2::{ZCompression, ZCompressionMode, ZMultiTokenTransferOutputData},
    state::{ZCompressedTokenMut, ZExtensionStructMut},
};
use light_program_profiler::profile;
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};
use spl_pod::solana_msg::msg;

use super::inputs::CompressAndCloseInputs;
use crate::{
    close_token_account::{
        accounts::CloseTokenAccountAccounts,
        processor::{close_token_account, validate_token_account_for_close_transfer2},
    },
    transfer2::accounts::Transfer2Accounts,
};

/// Process compress and close operation for a ctoken account
#[profile]
pub fn process_compress_and_close(
    authority: Option<&AccountInfo>,
    compress_and_close_inputs: Option<CompressAndCloseInputs>,
    amount: u64,
    token_account_info: &AccountInfo,
    ctoken: &mut ZCompressedTokenMut,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
) -> Result<Option<u64>, ProgramError> {
    let authority = authority.ok_or(ErrorCode::CompressAndCloseAuthorityMissing)?;
    check_signer(authority).map_err(|e| {
        anchor_lang::solana_program::msg!("Authority signer check failed: {:?}", e);
        ProgramError::from(e)
    })?;

    let close_inputs =
        compress_and_close_inputs.ok_or(ErrorCode::CompressAndCloseDestinationMissing)?;

    let (compression_authority_is_signer, compress_to_pubkey) =
        validate_token_account_for_close_transfer2(
            &CloseTokenAccountAccounts {
                token_account: token_account_info,
                destination: close_inputs.destination,
                authority,
                rent_sponsor: Some(close_inputs.rent_sponsor),
            },
            ctoken,
        )?;

    if compression_authority_is_signer {
        // Compress the complete balance to this compressed token account.
        validate_compressed_token_account(
            packed_accounts,
            amount,
            close_inputs.compressed_token_account,
            ctoken,
            compress_to_pubkey,
            token_account_info.key(),
        )?;
    }

    *ctoken.amount = 0.into();
    Ok(None)
}

/// Validate compressed token account for compress and close operation
fn validate_compressed_token_account(
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
    compression_amount: u64,
    compressed_token_account: &ZMultiTokenTransferOutputData<'_>,
    ctoken: &ZCompressedTokenMut,
    compress_to_pubkey: bool,
    token_account_pubkey: &Pubkey,
) -> Result<(), ProgramError> {
    // Owners should match if not compressing to pubkey
    if compress_to_pubkey {
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
    } else if *ctoken.owner
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
    if *ctoken.amount != compressed_token_account.amount {
        msg!(
            "output ctoken.amount {} != compressed token account amount {}",
            ctoken.amount,
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
    // Version should be ShaFlat
    if compressed_token_account.version != 3 {
        return Err(ErrorCode::CompressAndCloseInvalidVersion.into());
    }

    // Version should also match what's specified in the compressible extension
    let expected_version = ctoken
        .extensions
        .as_ref()
        .and_then(|ext| {
            if let Some(ZExtensionStructMut::Compressible(ext)) = ext.first() {
                Some(ext.account_version)
            } else {
                None
            }
        })
        .ok_or(ErrorCode::CompressAndCloseInvalidVersion)?;

    if compressed_token_account.version != expected_version {
        return Err(ErrorCode::CompressAndCloseInvalidVersion.into());
    }
    Ok(())
}

/// Close ctoken accounts after compress and close operations
pub fn close_for_compress_and_close(
    compressions: &[ZCompression<'_>],
    validated_accounts: &Transfer2Accounts,
) -> Result<(), ProgramError> {
    for compression in compressions
        .iter()
        .filter(|c| c.mode == ZCompressionMode::CompressAndClose)
    {
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
        close_token_account(&CloseTokenAccountAccounts {
            token_account: token_account_info,
            destination,
            authority,
            rent_sponsor: Some(rent_sponsor),
        })?;
    }
    Ok(())
}
