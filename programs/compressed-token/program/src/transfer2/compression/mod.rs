use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use arrayvec::ArrayVec;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_compressed_account::pubkey::AsPubkey;
use light_ctoken_types::instructions::transfer2::{
    ZCompressedTokenInstructionDataTransfer2, ZCompression, ZCompressionMode,
};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;

use crate::{
    shared::{
        convert_program_error,
        transfer_lamports::{multi_transfer_lamports, Transfer},
    },
    LIGHT_CPI_SIGNER,
};

pub mod ctoken;
pub mod spl;

pub use ctoken::{
    close_for_compress_and_close, compress_or_decompress_ctokens, CTokenCompressionInputs,
};

const SPL_TOKEN_ID: &[u8; 32] = &spl_token::ID.to_bytes();
const SPL_TOKEN_2022_ID: &[u8; 32] = &spl_token_2022::ID.to_bytes();
const ID: &[u8; 32] = &LIGHT_CPI_SIGNER.program_id;

/// Process native compressions/decompressions with token accounts
#[profile]
pub fn process_token_compression(
    fee_payer: &AccountInfo,
    inputs: &ZCompressedTokenInstructionDataTransfer2,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
    cpi_authority: &AccountInfo,
) -> Result<(), ProgramError> {
    if let Some(compressions) = inputs.compressions.as_ref() {
        // Array to accumulate transfer amounts by account index (max 40 packed accounts)
        let mut transfer_map = [0u64; 40];

        for compression in compressions {
            let source_or_recipient = packed_accounts.get_u8(
                compression.source_or_recipient,
                "compression source or recipient",
            )?;

            let transfer = match source_or_recipient.owner() {
                ID => ctoken::process_ctoken_compressions(
                    inputs,
                    compression,
                    source_or_recipient,
                    packed_accounts,
                )?,
                SPL_TOKEN_ID => {
                    spl::process_spl_compressions(
                        compression,
                        &SPL_TOKEN_ID.to_pubkey_bytes(),
                        source_or_recipient,
                        packed_accounts,
                        cpi_authority,
                    )?;
                    // SPL token compressions don't require lamport transfers for compressible extension´
                    None
                }
                SPL_TOKEN_2022_ID => {
                    spl::process_spl_compressions(
                        compression,
                        &SPL_TOKEN_2022_ID.to_pubkey_bytes(),
                        source_or_recipient,
                        packed_accounts,
                        cpi_authority,
                    )?;
                    // SPL token compressions don't require lamport transfers for compressible extension´
                    None
                }
                _ => {
                    msg!(
                        "source_or_recipient {:?}",
                        solana_pubkey::Pubkey::new_from_array(*source_or_recipient.key())
                    );
                    msg!(
                        "Invalid token program ID {:?}",
                        solana_pubkey::Pubkey::from(*source_or_recipient.owner())
                    );
                    return Err(ProgramError::InvalidInstructionData);
                }
            };

            // Accumulate transfer amount if present
            if let Some((account_index, amount)) = transfer {
                if account_index > 40 {
                    msg!(
                        "Too many compression transfers: {}, max 40 allowed",
                        account_index
                    );
                    return Err(ErrorCode::TooManyCompressionTransfers.into());
                }
                transfer_map[account_index as usize] = transfer_map[account_index as usize]
                    .checked_add(amount)
                    .ok_or(ProgramError::ArithmeticOverflow)?;
            }
        }

        // Build rent_return_transfers & top up array from accumulated amounts
        let transfers: ArrayVec<Transfer, 40> = transfer_map
            .iter()
            .enumerate()
            .filter_map(|(index, &amount)| {
                if amount != 0 {
                    Some((index as u8, amount))
                } else {
                    None
                }
            })
            .map(|(index, amount)| {
                Ok(Transfer {
                    account: packed_accounts.get_u8(index, "transfer account")?,
                    amount,
                })
            })
            .collect::<Result<ArrayVec<Transfer, 40>, ProgramError>>()?;

        if !transfers.is_empty() {
            multi_transfer_lamports(fee_payer, &transfers).map_err(convert_program_error)?
        }
    }
    Ok(())
}

/// Validate compression fields based on compression mode
#[profile]
#[inline(always)]
pub(crate) fn validate_compression_mode_fields(
    compression: &ZCompression,
) -> Result<(), ProgramError> {
    match compression.mode {
        ZCompressionMode::Decompress => {
            // the authority field is not used.
            if compression.authority != 0 {
                msg!("authority must be 0 for Decompress mode");
                return Err(ProgramError::InvalidInstructionData);
            }
        }
        ZCompressionMode::Compress | ZCompressionMode::CompressAndClose => {
            // No additional validation needed for regular compress
        }
    }

    Ok(())
}
