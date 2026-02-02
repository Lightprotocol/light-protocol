//! Pack and Unpack traits for converting between full Pubkeys and u8 indices.

use light_account_checks::AccountInfoTrait;

use crate::error::LightSdkTypesError;

#[cfg(all(not(target_os = "solana"), feature = "std"))]
use light_account_checks::AccountMetaTrait;

/// Replace 32-byte Pubkeys with 1-byte indices to save space.
/// If your type has no Pubkeys, just return self.
#[cfg(all(not(target_os = "solana"), feature = "std"))]
pub trait Pack<AM: AccountMetaTrait> {
    type Packed: crate::AnchorSerialize + Clone + core::fmt::Debug;

    fn pack(
        &self,
        remaining_accounts: &mut crate::interface::instruction::PackedAccounts<AM>,
    ) -> Result<Self::Packed, LightSdkTypesError>;
}

pub trait Unpack<AI: AccountInfoTrait> {
    type Unpacked;

    fn unpack(&self, remaining_accounts: &[AI]) -> Result<Self::Unpacked, LightSdkTypesError>;
}
