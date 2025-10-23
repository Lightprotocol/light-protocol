#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;
#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

use pinocchio::instruction::AccountMeta;

/// Trait for types that can provide account information for CPI calls
pub trait CpiAccountsTrait {
    /// Convert to a vector of AccountMeta references for instruction
    fn to_account_metas(&self) -> crate::error::Result<Vec<AccountMeta<'_>>>;

    /// Convert to account infos for invoke
    fn to_account_infos_for_invoke(
        &self,
    ) -> crate::error::Result<Vec<&pinocchio::account_info::AccountInfo>>;

    /// Get the CPI signer bump
    fn bump(&self) -> u8;

    /// Get the mode for the instruction (0 for v1, 1 for v2)
    fn get_mode(&self) -> u8;
}
