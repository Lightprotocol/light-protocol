//! Optimized top-up lamports calculation for CMint accounts.

use light_compressible::compression_info::CompressionInfo;
use light_program_profiler::profile;
use light_zero_copy::traits::ZeroCopyAt;
#[cfg(target_os = "solana")]
use pinocchio::account_info::AccountInfo;

use super::compressed_mint::ACCOUNT_TYPE_MINT;

/// Minimum size for CMint with CompressionInfo.
/// 166 (offset to CompressionInfo) + 96 (CompressionInfo size) = 262
pub const CMINT_MIN_SIZE_WITH_COMPRESSION: usize = COMPRESSION_INFO_OFFSET + COMPRESSION_INFO_SIZE;

/// Offset to CompressionInfo in CMint.
/// 82 (BaseMint) + 66 (metadata) + 17 (reserved) + 1 (account_type) = 166
const COMPRESSION_INFO_OFFSET: usize = 166;

/// Size of CompressionInfo struct (96 bytes).
const COMPRESSION_INFO_SIZE: usize = 96;

/// Offset to account_type field.
const ACCOUNT_TYPE_OFFSET: usize = 165;

/// Calculate top-up lamports directly from CMint account bytes.
/// Returns None if account is not a valid CMint.
#[inline(always)]
#[profile]
pub fn cmint_top_up_lamports_from_slice(
    data: &[u8],
    current_lamports: u64,
    current_slot: u64,
) -> Option<u64> {
    if data.len() < CMINT_MIN_SIZE_WITH_COMPRESSION
        || data[ACCOUNT_TYPE_OFFSET] != ACCOUNT_TYPE_MINT
    {
        return None;
    }

    let (info, _) = CompressionInfo::zero_copy_at(
        &data[COMPRESSION_INFO_OFFSET..COMPRESSION_INFO_OFFSET + COMPRESSION_INFO_SIZE],
    )
    .ok()?;

    info.calculate_top_up_lamports(data.len() as u64, current_slot, current_lamports)
        .ok()
}

/// Calculate top-up lamports from a CMint AccountInfo.
/// Verifies account owner is the CToken program. Returns None if owner mismatch or invalid.
/// Pass `current_slot` as 0 to fetch from Clock sysvar; non-zero values are used directly.
#[cfg(target_os = "solana")]
#[inline(always)]
#[profile]
pub fn cmint_top_up_lamports_from_account_info(
    account_info: &AccountInfo,
    current_slot: &mut u64,
) -> Option<u64> {
    use pinocchio::sysvars::{clock::Clock, Sysvar};

    // Check owner is CToken program
    if !account_info.is_owned_by(&crate::CTOKEN_PROGRAM_ID) {
        return None;
    }

    let data = account_info.try_borrow_data().ok()?;

    if data.len() < CMINT_MIN_SIZE_WITH_COMPRESSION
        || data[ACCOUNT_TYPE_OFFSET] != ACCOUNT_TYPE_MINT
    {
        return None;
    }

    let current_lamports = account_info.lamports();
    if *current_slot == 0 {
        *current_slot = Clock::get().ok()?.slot;
    }

    let (info, _) = CompressionInfo::zero_copy_at(
        &data[COMPRESSION_INFO_OFFSET..COMPRESSION_INFO_OFFSET + COMPRESSION_INFO_SIZE],
    )
    .ok()?;

    info.calculate_top_up_lamports(data.len() as u64, *current_slot, current_lamports)
        .ok()
}
