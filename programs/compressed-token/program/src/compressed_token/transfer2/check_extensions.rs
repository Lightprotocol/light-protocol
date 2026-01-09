use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_array_map::ArrayMap;
use light_ctoken_interface::{
    instructions::{
        extensions::ZExtensionInstructionData, transfer2::ZCompressedTokenInstructionDataTransfer2,
    },
    state::TokenDataVersion,
};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;

use crate::extensions::{check_mint_extensions, parse_mint_extensions, MintExtensionChecks};

/// Validate TLV data and extract is_frozen flag from CompressedOnly extension.
///
/// Returns error if TLV data is present but version is not 3 (ShaFlat).
/// Returns the is_frozen flag from CompressedOnly extension, or false if not present.
#[inline(always)]
pub fn validate_tlv_and_get_frozen(
    tlv_data: Option<&[ZExtensionInstructionData]>,
    version: u8,
) -> Result<bool, ProgramError> {
    // Validate TLV is only used with version 3 (ShaFlat)
    if tlv_data.is_some_and(|v| !v.is_empty() && version != TokenDataVersion::ShaFlat as u8) {
        msg!("TLV extensions only supported with version 3 (ShaFlat)");
        return Err(ErrorCode::TlvRequiresVersion3.into());
    }

    // Extract is_frozen from CompressedOnly extension (0 = false, non-zero = true)
    let is_frozen = tlv_data
        .and_then(|exts| {
            exts.iter().find_map(|ext| {
                if let ZExtensionInstructionData::CompressedOnly(data) = ext {
                    Some(data.is_frozen != 0)
                } else {
                    None
                }
            })
        })
        .unwrap_or(false);

    Ok(is_frozen)
}

/// Cache for mint extension checks to avoid deserializing the same mint multiple times.
pub type MintExtensionCache = ArrayMap<u8, MintExtensionChecks, 5>;

/// Build mint extension cache for all unique mints in the instruction.
///
/// # Extension State Enforcement Strategy
///
/// Restrictions (paused, non-zero fees, non-nil hook) are enforced when **entering** compressed
/// state, not when **exiting** it. This protects users who compressed tokens before restrictions
/// were added - they can always recover their tokens.
///
/// - **Compress** (from ctoken or SPL): Enforces restrictions. Creating new compressed state
///   requires valid extension state.
/// - **Decompress**: Bypasses restrictions. Restoring existing compressed state to on-chain.
///   If mint state changed after compression, user should still recover their tokens.
/// - **CompressAndClose**: Bypasses restrictions. Preserving state in CompressedOnly extension.
///   Foresters should be able to close accounts to recover rent exemption even if mint state changed.
///
/// # Errors (Compress mode only):
/// - `MintPaused` - Mint is paused
/// - `NonZeroTransferFeeNotSupported` - Transfer fees are non-zero
/// - `TransferHookNotSupported` - Transfer hook program_id is non-nil
/// - `MintHasRestrictedExtensions` - When `deny_restricted_extensions=true` and mint has
///   Pausable, PermanentDelegate, TransferFeeConfig, TransferHook, or DefaultAccountState extensions
///
/// # Cached data:
/// - `permanent_delegate`: Pubkey if PermanentDelegate extension exists and is set
/// - `has_transfer_fee`: Whether TransferFeeConfig extension exists
/// - `has_restricted_extensions`: Whether mint has restricted extensions
/// - `is_paused`, `has_non_zero_transfer_fee`, `has_non_nil_transfer_hook`: Individual state flags
#[profile]
#[inline(always)]
pub fn build_mint_extension_cache<'a>(
    inputs: &ZCompressedTokenInstructionDataTransfer2,
    packed_accounts: &'a ProgramPackedAccounts<'a, AccountInfo>,
) -> Result<MintExtensionCache, ProgramError> {
    let mut cache: MintExtensionCache = ArrayMap::new();
    let no_compressed_outputs = inputs.out_token_data.is_empty();
    let deny_restricted_extensions = !no_compressed_outputs;

    // Collect mints from input token data
    for input in inputs.in_token_data.iter() {
        let mint_index = input.mint;
        if cache.get_by_key(&mint_index).is_none() {
            let mint_account = packed_accounts.get_u8(mint_index, "mint cache: input")?;
            let checks = if no_compressed_outputs {
                // No outputs - bypass state checks (full decompress or transfer-only)
                parse_mint_extensions(mint_account)?
            } else {
                check_mint_extensions(mint_account, deny_restricted_extensions)?
            };
            cache.insert(mint_index, checks, ErrorCode::MintCacheCapacityExceeded)?;
        }
    }

    // Collect mints from compressions
    if let Some(compressions) = inputs.compressions.as_ref() {
        for compression in compressions.iter() {
            let mint_index = compression.mint;
            if cache.get_by_key(&mint_index).is_none() {
                let mint_account = packed_accounts.get_u8(mint_index, "mint cache: compression")?;
                let checks = if compression.mode.is_compress_and_close() || no_compressed_outputs {
                    // Bypass extension state checks (paused, non-zero fees, non-nil transfer hook)
                    // when CompressAndClose, full Decompress, or CToken→SPL (compress and full decompress)
                    parse_mint_extensions(mint_account)?
                } else {
                    check_mint_extensions(mint_account, deny_restricted_extensions)?
                };
                cache.insert(mint_index, checks, ErrorCode::MintCacheCapacityExceeded)?;
            }

            // SAFETY: mint_index was just inserted above if not already present
            let checks = cache.get_by_key(&mint_index).unwrap();
            // CompressAndClose with restricted extensions requires CompressedOnly output.
            // Compress/Decompress don't need additional validation here:
            // - Compress: blocked by check_mint_extensions when outputs exist
            // - Decompress: no check it restores existing state
            if checks.has_restricted_extensions && compression.mode.is_compress_and_close() {
                let output_idx = compression.get_compressed_token_account_index()?;
                let has_compressed_only = inputs
                    .out_tlv
                    .as_ref()
                    .and_then(|tlvs| tlvs.get(output_idx as usize))
                    .map(|tlv| {
                        tlv.iter()
                            .any(|e| matches!(e, ZExtensionInstructionData::CompressedOnly(_)))
                    })
                    .unwrap_or(false);
                if !has_compressed_only {
                    msg!("Mint has restricted extensions - CompressedOnly output required");
                    return Err(ErrorCode::CompressAndCloseMissingCompressedOnlyExtension.into());
                }
            }
        }
    }

    for output in inputs.out_token_data.iter() {
        // All mints of outputs that have non zero amount must have an input or compression.
        // Thus we only check outputs with zero amounts here.
        if output.amount.get() == 0 {
            let mint_index = output.mint;
            if cache.get_by_key(&mint_index).is_none() {
                let mint_account = packed_accounts.get_u8(mint_index, "mint cache: output")?;
                let checks = check_mint_extensions(mint_account, true)?;
                cache.insert(mint_index, checks, ErrorCode::MintCacheCapacityExceeded)?;
            }
        }
    }

    Ok(cache)
}
