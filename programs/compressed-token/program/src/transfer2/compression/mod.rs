use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use arrayvec::ArrayVec;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_compressed_account::pubkey::AsPubkey;
use light_ctoken_interface::{
    instructions::transfer2::{
        ZCompressedTokenInstructionDataTransfer2, ZCompression, ZCompressionMode,
    },
    CTokenError,
};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;

use super::check_extensions::MintExtensionCache;
use crate::{
    shared::{
        convert_program_error,
        transfer_lamports::{multi_transfer_lamports, Transfer},
    },
    LIGHT_CPI_SIGNER, MAX_PACKED_ACCOUNTS,
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
///
/// # Arguments
/// * `max_top_up` - Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
#[profile]
pub fn process_token_compression<'a>(
    fee_payer: &AccountInfo,
    inputs: &'a ZCompressedTokenInstructionDataTransfer2<'a>,
    packed_accounts: &'a ProgramPackedAccounts<'a, AccountInfo>,
    cpi_authority: &AccountInfo,
    max_top_up: u16,
    mint_cache: &'a MintExtensionCache,
) -> Result<(), ProgramError> {
    if let Some(compressions) = inputs.compressions.as_ref() {
        let mut transfer_map = [0u64; MAX_PACKED_ACCOUNTS];
        // Initialize budget: +1 allows exact match (total == max_top_up)
        let mut lamports_budget = (max_top_up as u64).saturating_add(1);

        for compression in compressions {
            let account_index = compression.source_or_recipient as usize;
            if account_index >= MAX_PACKED_ACCOUNTS {
                msg!(
                    "Account index {} out of bounds, max {} allowed",
                    account_index,
                    MAX_PACKED_ACCOUNTS
                );
                return Err(ErrorCode::TooManyCompressionTransfers.into());
            }

            let source_or_recipient = packed_accounts.get_u8(
                compression.source_or_recipient,
                "compression source or recipient",
            )?;

            // Lookup cached mint extension checks (cache was built with skip logic already applied)
            let mint_checks = mint_cache.get_by_key(&compression.mint).cloned();

            match source_or_recipient.owner() {
                ID => ctoken::process_ctoken_compressions(
                    inputs,
                    compression,
                    source_or_recipient,
                    packed_accounts,
                    mint_checks,
                    &mut transfer_map[account_index],
                    &mut lamports_budget,
                )?,
                SPL_TOKEN_ID => {
                    spl::process_spl_compressions(
                        compression,
                        &SPL_TOKEN_ID.to_pubkey_bytes(),
                        source_or_recipient,
                        packed_accounts,
                        cpi_authority,
                    )?;
                }
                SPL_TOKEN_2022_ID => {
                    spl::process_spl_compressions(
                        compression,
                        &SPL_TOKEN_2022_ID.to_pubkey_bytes(),
                        source_or_recipient,
                        packed_accounts,
                        cpi_authority,
                    )?;
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
        }

        let transfers: ArrayVec<Transfer, MAX_PACKED_ACCOUNTS> = transfer_map
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
            .collect::<Result<ArrayVec<Transfer, MAX_PACKED_ACCOUNTS>, ProgramError>>()?;

        if !transfers.is_empty() {
            // Check budget wasn't exhausted (0 means exceeded max_top_up)
            if max_top_up != 0 && lamports_budget == 0 {
                return Err(CTokenError::MaxTopUpExceeded.into());
            }
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
