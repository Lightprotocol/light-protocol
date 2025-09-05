use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize};

pub const SLOTS_PER_EPOCH: u64 = 432_000;
// TODO: add token account version
// TODO: consider adding externally funded mode
/// Compressible extension for token accounts
/// Contains timing data for compression/decompression and rent authority
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
#[repr(C)]
pub struct CompressibleExtension {
    pub version: u8, // version 0 is uninitialized, default is 1
    pub rent_authority: Option<[u8; 32]>,
    pub rent_recipient: Option<[u8; 32]>,
    pub last_claimed_slot: u64,
    pub lamports_at_last_claimed_slot: u64,
    pub write_top_up_lamports: Option<u32>,
}

pub const MIN_RENT: u64 = 1220;

pub fn rent_curve_per_epoch(bytes: u64) -> u64 {
    MIN_RENT + bytes * 10
}

pub fn get_rent(bytes: u64, epochs: u64) -> u64 {
    rent_curve_per_epoch(bytes) * epochs
}
impl CompressibleExtension {
    /// current - last epoch = num epochs due
    /// rent_due
    /// available_balance = current_lamports - last_lamports
    ///     (we can never claim more lamports than rent is due)
    /// remaining_balance = available_balance - rent_due
    pub fn is_compressible(&self, bytes: u64, current_slot: u64, current_lamports: u64) -> bool {
        let current_epoch = current_slot / SLOTS_PER_EPOCH;
        let last_claimed_epoch = self.last_claimed_slot / SLOTS_PER_EPOCH;
        let num_epochs_due = current_epoch.checked_sub(last_claimed_epoch).unwrap(); // unwrap is a bug it should not happen
        let rent_due = get_rent(bytes, num_epochs_due);
        let available_balance = current_lamports
            .checked_sub(self.lamports_at_last_claimed_slot)
            .unwrap(); // unwrap is a bug it should not happen
        available_balance < rent_due
    }
}
impl ZCompressibleExtension<'_> {
    /// current - last epoch = num epochs due
    /// rent_due
    /// available_balance = current_lamports - last_lamports
    ///     (we can never claim more lamports than rent is due)
    /// remaining_balance = available_balance - rent_due
    pub fn is_compressible(&self, bytes: u64, current_slot: u64, current_lamports: u64) -> bool {
        let current_epoch = current_slot / SLOTS_PER_EPOCH;
        let last_claimed_epoch = self.last_claimed_slot.get() / SLOTS_PER_EPOCH;
        let num_epochs_due = current_epoch.checked_sub(last_claimed_epoch).unwrap(); // unwrap is a bug it should not happen
        let rent_due = get_rent(bytes, num_epochs_due);
        let available_balance = current_lamports
            .checked_sub(self.lamports_at_last_claimed_slot.get())
            .unwrap(); // unwrap is a bug it should not happen
        available_balance < rent_due
    }
}
impl ZCompressibleExtensionMut<'_> {
    /// current - last epoch = num epochs due
    /// rent_due
    /// available_balance = current_lamports - last_lamports
    ///     (we can never claim more lamports than rent is due)
    /// remaining_balance = available_balance - rent_due
    pub fn is_compressible(&self, bytes: u64, current_slot: u64, current_lamports: u64) -> bool {
        let current_epoch = current_slot / SLOTS_PER_EPOCH;
        let last_claimed_epoch = self.last_claimed_slot.get() / SLOTS_PER_EPOCH;
        let num_epochs_due = current_epoch.checked_sub(last_claimed_epoch).unwrap(); // unwrap is a bug it should not happen
        let rent_due = get_rent(bytes, num_epochs_due);
        let available_balance = current_lamports
            .checked_sub(self.lamports_at_last_claimed_slot.get())
            .unwrap(); // unwrap is a bug it should not happen
        available_balance < rent_due
    }
}
// impl ZCompressibleExtensionMut<'_> {
//     /// Get the remaining slots until compression is allowed
//     /// Returns 0 if compression is already allowed
//     #[cfg(target_os = "solana")]
//     pub fn remaining_slots(&self) -> Result<u64, crate::CTokenError> {
//         let current_slot = {
//             use pinocchio::sysvars::{clock::Clock, Sysvar};
//             Clock::get()
//                 .map_err(|_| crate::CTokenError::SysvarAccessError)?
//                 .slot
//         };
//         let target_slot = self.last_written_slot + self.slots_until_compression;
//         Ok(u64::from(target_slot).saturating_sub(current_slot))
//     }

//     // Note this might clash with rust tests. (Maybe I can use an env variable)
//     /// Get the remaining slots until compression is allowed (non-Solana target)
//     /// Returns 0 if compression is already allowed
//     #[cfg(not(target_os = "solana"))]
//     pub fn remaining_slots(&self, current_slot: u64) -> u64 {
//         let target_slot = self.last_written_slot + self.slots_until_compression;
//         u64::from(target_slot).saturating_sub(current_slot)
//     }

//     /// Check if the account is compressible (timing constraints have elapsed)
//     #[cfg(target_os = "solana")]
//     pub fn is_compressible(&self) -> Result<bool, crate::CTokenError> {
//         Ok(self.remaining_slots()? == 0)
//     }

//     /// Check if the account is compressible (timing constraints have elapsed) - non-Solana target
//     #[cfg(not(target_os = "solana"))]
//     pub fn is_compressible(&self, current_slot: u64) -> bool {
//         self.remaining_slots(current_slot) == 0
//     }
// }
