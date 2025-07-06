use anchor_compressed_token::ErrorCode;
use arrayvec::ArrayVec;

use crate::multi_transfer::instruction_data::{
    ZCompression, ZMultiInputTokenDataWithContext, ZMultiTokenTransferOutputData,
};

/// Process inputs and add amounts to mint sums with order validation
#[inline(always)]
fn sum_inputs(
    inputs: &[ZMultiInputTokenDataWithContext],
    mint_sums: &mut ArrayVec<(u8, u64), 5>,
) -> Result<(), ErrorCode> {
    let mut prev_mint_index = 0u8;
    for (i, input) in inputs.iter().enumerate() {
        let mint_index = input.mint;

        // Validate incremental order
        if i > 0 && mint_index < prev_mint_index {
            return Err(ErrorCode::InputsOutOfOrder);
        }

        // Find or create mint entry
        if let Some(entry) = mint_sums.iter_mut().find(|(idx, _)| *idx == mint_index) {
            entry.1 = entry
                .1
                .checked_add(input.amount.into())
                .ok_or(ErrorCode::ComputeInputSumFailed)?;
        } else {
            if mint_sums.is_full() {
                return Err(ErrorCode::TooManyMints);
            }
            mint_sums.push((mint_index, input.amount.into()));
        }

        prev_mint_index = mint_index;
    }
    Ok(())
}

/// Process compressions and adjust mint sums (add for compress, subtract for decompress)
#[inline(always)]
fn sum_compressions(
    compressions: &[ZCompression],
    mint_sums: &mut ArrayVec<(u8, u64), 5>,
) -> Result<(), ErrorCode> {
    for compression in compressions.iter() {
        let mint_index = compression.mint;

        // Find mint entry (create if doesn't exist for compression)
        if let Some(entry) = mint_sums.iter_mut().find(|(idx, _)| *idx == mint_index) {
            if compression.is_compress() {
                // Compress: add to balance
                entry.1 = entry
                    .1
                    .checked_add(compression.amount.into())
                    .ok_or(ErrorCode::ComputeCompressSumFailed)?;
            } else {
                // Decompress: subtract from balance
                entry.1 = entry
                    .1
                    .checked_sub(compression.amount.into())
                    .ok_or(ErrorCode::ComputeDecompressSumFailed)?;
            }
        } else {
            // Create new entry if compressing
            if compression.is_compress() {
                if mint_sums.is_full() {
                    return Err(ErrorCode::TooManyMints);
                }
                mint_sums.push((mint_index, compression.amount.into()));
            } else {
                // Cannot decompress if no balance exists
                return Err(ErrorCode::SumCheckFailed);
            }
        }
    }
    Ok(())
}

/// Process outputs and subtract amounts from mint sums
#[inline(always)]
fn sum_outputs(
    outputs: &[ZMultiTokenTransferOutputData],
    mint_sums: &mut ArrayVec<(u8, u64), 5>,
) -> Result<(), ErrorCode> {
    for output in outputs.iter() {
        let mint_index = output.mint;

        // Find mint entry (create if doesn't exist for output-only mints)
        if let Some(entry) = mint_sums.iter_mut().find(|(idx, _)| *idx == mint_index) {
            entry.1 = entry
                .1
                .checked_sub(output.amount.into())
                .ok_or(ErrorCode::ComputeOutputSumFailed)?;
        } else {
            // Output mint not in inputs or compressions - invalid
            return Err(ErrorCode::SumCheckFailed);
        }
    }
    Ok(())
}

/// Sum check for multi-mint transfers with ordered mint validation and compression support
pub fn sum_check_multi_mint(
    inputs: &[ZMultiInputTokenDataWithContext],
    outputs: &[ZMultiTokenTransferOutputData],
    compressions: Option<&[ZCompression]>,
) -> Result<(), ErrorCode> {
    // ArrayVec with 5 entries: (mint_index, sum)
    let mut mint_sums: ArrayVec<(u8, u64), 5> = ArrayVec::new();

    // Process inputs - increase sums
    sum_inputs(inputs, &mut mint_sums)?;

    // Process compressions if present
    if let Some(compressions) = compressions {
        sum_compressions(compressions, &mut mint_sums)?;
    }

    // Process outputs - decrease sums
    sum_outputs(outputs, &mut mint_sums)?;

    // Verify all sums are zero
    for (_, sum) in mint_sums.iter() {
        if *sum != 0 {
            return Err(ErrorCode::SumCheckFailed);
        }
    }

    Ok(())
}
