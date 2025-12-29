use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize};

/// Extension indicating the account belongs to a mint with transfer hook.
/// Contains a `transferring` flag used as a reentrancy guard during hook CPI.
/// Consistent with SPL Token-2022 TransferHookAccount layout.
#[derive(
    Debug,
    Clone,
    Copy,
    Hash,
    PartialEq,
    Eq,
    Default,
    AnchorSerialize,
    AnchorDeserialize,
    ZeroCopy,
    ZeroCopyMut,
)]
#[repr(C)]
pub struct TransferHookAccountExtension {
    /// Flag to indicate that the account is in the middle of a transfer.
    /// Used as reentrancy guard when transfer hook program is called via CPI.
    /// Always false at rest since we only support nil program_id (no hook invoked).
    pub transferring: u8,
}
