//! Pack and Unpack traits for converting between full Pubkeys and u8 indices.

use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;

#[cfg(not(target_os = "solana"))]
use crate::instruction::PackedAccounts;
#[cfg(not(target_os = "solana"))]
use crate::AnchorSerialize;

/// Replace 32-byte Pubkeys with 1-byte indices to save space.
/// If your type has no Pubkeys, just return self.
#[cfg(not(target_os = "solana"))]
pub trait Pack {
    type Packed: AnchorSerialize + Clone + std::fmt::Debug;

    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Result<Self::Packed, ProgramError>;
}

pub trait Unpack {
    type Unpacked;

    fn unpack(&self, remaining_accounts: &[AccountInfo]) -> Result<Self::Unpacked, ProgramError>;
}
