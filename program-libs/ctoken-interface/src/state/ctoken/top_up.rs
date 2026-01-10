//! Optimized top-up lamports calculation for CToken accounts.

use light_compressible::compression_info::CompressionInfo;
use light_program_profiler::profile;
#[cfg(target_os = "solana")]
use pinocchio::account_info::AccountInfo;

use super::ACCOUNT_TYPE_TOKEN_ACCOUNT;
use crate::state::ExtensionType;

/// Minimum size for CToken with Compressible extension as first extension.
/// 176 (offset to CompressionInfo) + 96 (CompressionInfo size) = 272
pub const MIN_SIZE_WITH_COMPRESSIBLE: usize = COMPRESSION_INFO_OFFSET + COMPRESSION_INFO_SIZE;

/// Offset to CompressionInfo when Compressible is first extension.
/// 165 (base) + 1 (account_type) + 1 (Option) + 4 (Vec len) + 1 (ext disc) + 4 (ext header) = 176
const COMPRESSION_INFO_OFFSET: usize = 176;

/// Size of CompressionInfo struct.
/// 2 (config_account_version) + 1 (compress_to_pubkey) + 1 (account_version) +
/// 4 (lamports_per_write) + 32 (compression_authority) + 32 (rent_sponsor) +
/// 8 (last_claimed_slot) + 4 (rent_exemption_paid) + 4 (_reserved) + 8 (rent_config) = 96
const COMPRESSION_INFO_SIZE: usize = 96;

/// Offset to account_type field.
const ACCOUNT_TYPE_OFFSET: usize = 165;

/// Offset to Option discriminator field.
const OPTION_DISCRIMINATOR_OFFSET: usize = 166;

/// Offset to first extension discriminator.
const FIRST_EXT_DISCRIMINATOR_OFFSET: usize = 171;

/// Option discriminator value for Some.
const OPTION_SOME: u8 = 1;

/// Calculate top-up lamports directly from CToken account bytes.
/// Returns None if account doesn't have Compressible extension as first extension.
#[inline(always)]
#[profile]
pub fn top_up_lamports_from_slice(
    data: &[u8],
    current_lamports: u64,
    current_slot: u64,
) -> Option<u64> {
    if data.len() < MIN_SIZE_WITH_COMPRESSIBLE
        || data[ACCOUNT_TYPE_OFFSET] != ACCOUNT_TYPE_TOKEN_ACCOUNT
        || data[OPTION_DISCRIMINATOR_OFFSET] != OPTION_SOME
        || data[FIRST_EXT_DISCRIMINATOR_OFFSET] != ExtensionType::Compressible as u8
    {
        return None;
    }

    let info: &CompressionInfo = bytemuck::from_bytes(
        &data[COMPRESSION_INFO_OFFSET..COMPRESSION_INFO_OFFSET + COMPRESSION_INFO_SIZE],
    );

    info.calculate_top_up_lamports(data.len() as u64, current_slot, current_lamports)
        .ok()
}

/// Calculate top-up lamports from an AccountInfo.
/// Returns None if account doesn't have Compressible extension as first extension.
/// Note: Does not verify account owner. Fetches clock/rent sysvars internally if needed.
/// Pass `current_slot` as 0 to fetch from Clock sysvar; non-zero values are used directly.
#[cfg(target_os = "solana")]
#[inline(always)]
#[profile]
pub fn top_up_lamports_from_account_info_unchecked(
    account_info: &AccountInfo,
    current_slot: &mut u64,
) -> Option<u64> {
    use pinocchio::sysvars::{clock::Clock, Sysvar};
    let data = account_info.try_borrow_data().ok()?;
    let current_lamports = account_info.lamports();
    if *current_slot == 0 {
        *current_slot = Clock::get().ok()?.slot;
    }
    top_up_lamports_from_slice(&data, current_lamports, *current_slot)
}
