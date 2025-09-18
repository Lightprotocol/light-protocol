use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_account_checks::{
    checks::{check_owner, check_signer},
    packed_accounts::ProgramPackedAccounts,
};
use light_ctoken_types::{
    instructions::transfer2::{
        ZCompressedTokenInstructionDataTransfer2, ZCompression, ZCompressionMode,
        ZMultiTokenTransferOutputData,
    },
    state::{CompressedToken, ZCompressedTokenMut, ZExtensionStructMut},
    CTokenError,
};
use light_profiler::profile;
use light_zero_copy::traits::ZeroCopyAtMut;
use pinocchio::sysvars::{clock::Clock, Sysvar};
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};
use spl_pod::solana_msg::msg;

use super::validate_compression_mode_fields;
use crate::{
    close_token_account::{
        accounts::CloseTokenAccountAccounts, processor::validate_token_account_for_close_transfer2,
    },
    shared::owner_validation::verify_and_update_token_account_authority_with_compressed_token,
};

/// Compress and close specific inputs
pub struct CompressAndCloseInputs<'a> {
    pub destination: &'a AccountInfo,
    pub rent_sponsor: &'a AccountInfo,
    pub compressed_token_account: &'a ZMultiTokenTransferOutputData<'a>,
}

/// Input struct for native compression/decompression operations
pub struct NativeCompressionInputs<'a> {
    pub authority: Option<&'a AccountInfo>,
    pub compress_and_close_inputs: Option<CompressAndCloseInputs<'a>>,
    pub amount: u64,
    pub mint: Pubkey,
    pub token_account_info: &'a AccountInfo,
    pub mode: ZCompressionMode,
    pub packed_accounts: &'a ProgramPackedAccounts<'a, AccountInfo>,
}

impl<'a> NativeCompressionInputs<'a> {
    /// Constructor for compression operations from Transfer2 instruction
    pub fn from_compression(
        compression: &ZCompression,
        token_account_info: &'a AccountInfo,
        inputs: &'a ZCompressedTokenInstructionDataTransfer2,
        packed_accounts: &'a ProgramPackedAccounts<'a, AccountInfo>,
    ) -> Result<Self, ProgramError> {
        let authority_account = if compression.mode != ZCompressionMode::Decompress {
            Some(packed_accounts.get_u8(
                compression.authority,
                "process_native_compression: authority",
            )?)
        } else {
            None
        };

        let mint_account = *packed_accounts
            .get_u8(compression.mint, "process_native_compression: token mint")?
            .key();

        let compress_and_close_inputs = if compression.mode == ZCompressionMode::CompressAndClose {
            let compressed_token_account = inputs
                .out_token_data
                .get(compression.get_compressed_token_account_index()? as usize)
                .ok_or(CTokenError::AccountFrozen)?;
            Some(CompressAndCloseInputs {
                destination: packed_accounts.get_u8(
                    compression.get_destination_index()?,
                    "process_native_compression: destination",
                )?,
                rent_sponsor: packed_accounts.get_u8(
                    compression.get_rent_sponsor_index()?,
                    "process_native_compression: rent_sponsor",
                )?,
                compressed_token_account,
            })
        } else {
            None
        };

        Ok(Self {
            authority: authority_account,
            compress_and_close_inputs,
            amount: (*compression.amount).into(),
            mint: mint_account,
            token_account_info,
            mode: compression.mode.clone(),
            packed_accounts,
        })
    }

    /// Simple constructor for decompression-only operations (used in mint_to_decompressed)
    pub fn decompress_only(
        amount: u64,
        mint: Pubkey,
        token_account_info: &'a AccountInfo,
        packed_accounts: &'a ProgramPackedAccounts<'a, AccountInfo>,
    ) -> Self {
        Self {
            authority: None,
            compress_and_close_inputs: None,
            amount,
            mint,
            token_account_info,
            mode: ZCompressionMode::Decompress,
            packed_accounts,
        }
    }
}

/// Process compression/decompression for token accounts using zero-copy PodAccount
#[profile]
pub(super) fn process_native_compressions(
    inputs: &ZCompressedTokenInstructionDataTransfer2,
    compression: &ZCompression,
    token_account_info: &AccountInfo,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
) -> Result<Option<(u8, u64)>, ProgramError> {
    // Validate compression fields for the given mode
    validate_compression_mode_fields(compression)?;

    // Create inputs struct with all required accounts extracted
    let compression_inputs = NativeCompressionInputs::from_compression(
        compression,
        token_account_info,
        inputs,
        packed_accounts,
    )?;

    let transfer_amount = native_compression(compression_inputs)?;

    // Return account index and amount if there's a transfer needed
    Ok(transfer_amount.map(|amount| (compression.source_or_recipient, amount)))
}

/// Perform native compression/decompression on a token account
#[profile]
pub fn native_compression(inputs: NativeCompressionInputs) -> Result<Option<u64>, ProgramError> {
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

    let (mut ctoken, _) = CompressedToken::zero_copy_at_mut(&mut token_account_data)
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if &ctoken.mint.to_bytes() != &mint {
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
        ZCompressionMode::CompressAndClose => {
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
                    &ctoken,
                )?;
            if compression_authority_is_signer {
                // Compress the complete balance to this compressed token account.
                validate_compressed_token_account(
                    packed_accounts,
                    amount,
                    close_inputs.compressed_token_account,
                    &ctoken,
                    compress_to_pubkey,
                    token_account_info.key(),
                )?;
            }
            *ctoken.amount = 0.into();

            Ok(None)
        }
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

fn validate_compressed_token_account(
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
    compression_amount: u64,
    compressed_token_account: &ZMultiTokenTransferOutputData<'_>,
    ctoken: &ZCompressedTokenMut,
    compress_to_pubkey: bool,
    token_account_pubkey: &Pubkey,
) -> Result<(), ProgramError> {
    // Owners should match if not compressing to pubkey
    if !compress_to_pubkey
        && *ctoken.owner
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
    // Owner should match token account pubkey if compressing to pubkey
    if compress_to_pubkey
        && *packed_accounts
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
