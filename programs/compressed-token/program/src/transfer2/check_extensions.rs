use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_array_map::ArrayMap;
use light_ctoken_interface::instructions::{
    extensions::ZExtensionInstructionData,
    transfer2::{ZCompressedTokenInstructionDataTransfer2, ZCompressionMode},
};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;

use crate::extensions::{check_mint_extensions, MintExtensionChecks};

/// Cache for mint extension checks to avoid deserializing the same mint multiple times.
pub type MintExtensionCache = ArrayMap<u8, MintExtensionChecks, 5>;

/// Build mint extension cache for all unique mints in the instruction.
///
/// # Checks performed per mint (via `check_mint_extensions`):
/// - **Pausable**: Fails with `MintPaused` if mint is paused
/// - **Restricted extensions**: When `has_output_compressed_accounts=true`, fails with
///   `MintHasRestrictedExtensions` if mint has Pausable, PermanentDelegate, TransferFeeConfig,
///   or TransferHook extensions
/// - **TransferFeeConfig**: Fails with `NonZeroTransferFeeNotSupported` if fees are non-zero
/// - **TransferHook**: Fails with `TransferHookNotSupported` if program_id is non-nil
///
/// # Cached data:
/// - `permanent_delegate`: Pubkey if PermanentDelegate extension exists and is set
/// - `has_transfer_fee`: Whether TransferFeeConfig extension exists (non-zero fees are rejected)
/// - `has_restricted_extensions`: Whether mint has restricted extensions (for CompressAndClose validation)
#[profile]
#[inline(always)]
pub fn build_mint_extension_cache<'a>(
    inputs: &ZCompressedTokenInstructionDataTransfer2,
    packed_accounts: &'a ProgramPackedAccounts<'a, AccountInfo>,
    deny_restricted_extensions: bool, // true if has_output_compressed_accounts
) -> Result<MintExtensionCache, ProgramError> {
    let mut cache: MintExtensionCache = ArrayMap::new();

    // Collect mints from input token data
    for input in inputs.in_token_data.iter() {
        let mint_index = input.mint;
        if cache.get_by_key(&mint_index).is_none() {
            let mint_account = packed_accounts.get_u8(mint_index, "mint cache: input")?;
            let checks = check_mint_extensions(mint_account, deny_restricted_extensions)?;
            cache.insert(mint_index, checks, ErrorCode::MintCacheCapacityExceeded)?;
        }
    }

    // Collect mints from compressions
    if let Some(compressions) = inputs.compressions.as_ref() {
        for compression in compressions.iter() {
            let mint_index = compression.mint;

            if cache.get_by_key(&mint_index).is_none() {
                let mint_account = packed_accounts.get_u8(mint_index, "mint cache: compression")?;
                let checks = if compression.rent_sponsor_is_signer()
                    && compression.mode == ZCompressionMode::CompressAndClose
                {
                    check_mint_extensions(
                        mint_account,
                        false, // Allow restricted extensions, also if instruction has has_output_compressed_accounts
                    )?
                } else {
                    check_mint_extensions(mint_account, deny_restricted_extensions)?
                };

                // Validate mints with restricted extensions:
                // - CompressAndClose with rent_sponsor_is_signer: OK if output has CompressedOnly
                // - Compress: NOT allowed (mints with restricted extensions must not be compressed)
                // - Decompress: OK (no output compressed accounts, handled by check_restricted)
                if checks.has_restricted_extensions {
                    match compression.mode {
                        ZCompressionMode::CompressAndClose => {
                            // Verify output has CompressedOnly extension
                            let output_idx = compression.get_compressed_token_account_index()?;
                            let has_compressed_only = inputs
                                .out_tlv
                                .as_ref()
                                .and_then(|tlvs| tlvs.get(output_idx as usize))
                                .map(|tlv| {
                                    tlv.iter().any(|e| {
                                        matches!(e, ZExtensionInstructionData::CompressedOnly(_))
                                    })
                                })
                                .unwrap_or(false);
                            if !has_compressed_only {
                                msg!("Mint has restricted extensions - CompressedOnly output required");
                                return Err(
                                    ErrorCode::CompressAndCloseMissingCompressedOnlyExtension
                                        .into(),
                                );
                            }
                        }
                        ZCompressionMode::Compress => {
                            // msg!("Mints with restricted extensions cannot be compressed");
                            // return Err(ErrorCode::MintHasRestrictedExtensions.into());
                        }
                        ZCompressionMode::Decompress => {
                            // OK - if we reach here, has_output_compressed_accounts=false
                            // (otherwise check_mint_extensions would have failed earlier)
                        }
                    }
                }

                cache.insert(mint_index, checks, ErrorCode::MintCacheCapacityExceeded)?;
            }
        }
    }

    Ok(cache)
}
