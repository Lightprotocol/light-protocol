use anchor_compressed_token::ErrorCode;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_array_map::ArrayMap;
use light_ctoken_types::instructions::transfer2::{
    ZCompression, ZCompressionMode, ZMultiInputTokenDataWithContext, ZMultiTokenTransferOutputData,
};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;

/// Process inputs and add amounts to mint sums
#[inline(always)]
#[profile]
fn sum_inputs(
    inputs: &[ZMultiInputTokenDataWithContext],
    mint_sums: &mut ArrayMap<u8, u64, 5>,
) -> Result<(), ErrorCode> {
    for input in inputs.iter() {
        // Find or create mint entry
        if let Some(balance) = mint_sums.get_mut_by_key(&input.mint) {
            *balance = balance
                .checked_add(input.amount.into())
                .ok_or(ErrorCode::ComputeInputSumFailed)?;
        } else {
            mint_sums.insert(input.mint, input.amount.into(), ErrorCode::TooManyMints)?;
        }
    }
    Ok(())
}

/// Process compressions and adjust mint sums (add for compress, subtract for decompress)
#[inline(always)]
#[profile]
pub fn sum_compressions(
    compressions: &[ZCompression],
    mint_sums: &mut ArrayMap<u8, u64, 5>,
) -> Result<(), ErrorCode> {
    for compression in compressions.iter() {
        let mint_index = compression.mint;

        // Find mint entry (create if doesn't exist for compression)
        if let Some(balance) = mint_sums.get_mut_by_key(&mint_index) {
            *balance = compression
                .new_balance_compressed_account(*balance)
                .map_err(|_| ErrorCode::SumCheckFailed)?;
        } else {
            // Create new entry if compressing
            if compression.mode == ZCompressionMode::Compress
                || compression.mode == ZCompressionMode::CompressAndClose
            {
                mint_sums.insert(
                    mint_index,
                    (*compression.amount).into(),
                    ErrorCode::TooManyMints,
                )?;
            } else {
                msg!("Cannot decompress if no balance exists");
                return Err(ErrorCode::SumCheckFailed);
            }
        }
    }
    Ok(())
}

/// Process outputs and subtract amounts from mint sums
#[inline(always)]
#[profile]
fn sum_outputs(
    outputs: &[ZMultiTokenTransferOutputData],
    mint_sums: &mut ArrayMap<u8, u64, 5>,
) -> Result<(), ErrorCode> {
    for output in outputs.iter() {
        let mint_index = output.mint;

        // Find mint entry - must exist from inputs or compressions
        if let Some(balance) = mint_sums.get_mut_by_key(&mint_index) {
            *balance = balance
                .checked_sub(output.amount.into())
                .ok_or(ErrorCode::ComputeOutputSumFailed)?;
        } else {
            // Output mint not in inputs or compressions - invalid
            return Err(ErrorCode::ComputeOutputSumFailed);
        }
    }
    Ok(())
}

/// Sum check for multi-mint transfers with compression support
/// Returns the mint map for external validation
#[profile]
#[inline(always)]
pub fn sum_check_multi_mint(
    inputs: &[ZMultiInputTokenDataWithContext],
    outputs: &[ZMultiTokenTransferOutputData],
    compressions: Option<&[ZCompression]>,
) -> Result<ArrayMap<u8, u64, 5>, ErrorCode> {
    // ArrayMap with 5 entries: mint_index -> balance
    let mut mint_sums: ArrayMap<u8, u64, 5> = ArrayMap::new();

    // Process inputs - increase sums
    sum_inputs(inputs, &mut mint_sums)?;

    // Process compressions if present
    if let Some(compressions) = compressions {
        sum_compressions(compressions, &mut mint_sums)?;
    }

    // Process outputs - decrease sums
    sum_outputs(outputs, &mut mint_sums)?;

    // Verify all sums are zero
    for i in 0..mint_sums.len() {
        if let Some((_mint_index, balance)) = mint_sums.get(i) {
            if *balance != 0 {
                return Err(ErrorCode::SumCheckFailed);
            }
        }
    }

    Ok(mint_sums)
}

/// Validate that each mint index in the map references a unique mint pubkey
/// This prevents attacks where the same mint index could be reused to reference different mints
#[profile]
#[inline(always)]
pub fn validate_mint_uniqueness(
    mint_map: &ArrayMap<u8, u64, 5>,
    packed_accounts: &ProgramPackedAccounts<AccountInfo>,
) -> Result<(), ErrorCode> {
    // Build a map of mint_pubkey -> mint_index to check for duplicates
    let mut seen_pubkeys: ArrayMap<[u8; 32], u8, 5> = ArrayMap::new();

    for i in 0..mint_map.len() {
        if let Some((mint_index, _balance)) = mint_map.get(i) {
            // Get the mint account pubkey from packed accounts
            let mint_account = packed_accounts
                .get(*mint_index as usize, "mint")
                .map_err(|_| ErrorCode::DuplicateMint)?;
            let mint_pubkey = mint_account.key();

            // Check if we've seen this pubkey with a different index
            if let Some(existing_index) = seen_pubkeys.get_by_pubkey(mint_pubkey) {
                // Same pubkey referenced by different index - this is an attack
                if *existing_index != *mint_index {
                    msg!(
                        "Duplicate mint detected: index {} and {} both reference the same mint pubkey",
                        existing_index,
                        mint_index
                    );
                    return Err(ErrorCode::DuplicateMint);
                }
            } else {
                // First time seeing this pubkey, record it
                seen_pubkeys.insert(*mint_pubkey, *mint_index, ErrorCode::TooManyMints)?;
            }
        }
    }

    Ok(())
}
