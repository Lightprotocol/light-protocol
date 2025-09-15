use anchor_lang::prelude::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_compressed_account::pubkey::AsPubkey;
use light_ctoken_types::instructions::transfer2::{
    ZCompressedTokenInstructionDataTransfer2, ZCompression, ZCompressionMode,
};
use light_profiler::profile;
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;

use crate::LIGHT_CPI_SIGNER;

pub mod native;
pub mod spl;

pub use native::native_compression;

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
        for compression in compressions {
            let source_or_recipient = packed_accounts.get_u8(
                compression.source_or_recipient,
                "compression source or recipient",
            )?;

            match source_or_recipient.owner() {
                ID => {
                    native::process_native_compressions(
                        fee_payer,
                        inputs,
                        compression,
                        source_or_recipient,
                        packed_accounts,
                    )?;
                }
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
            }
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
