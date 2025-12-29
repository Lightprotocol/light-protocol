use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize};

/// Marker extension indicating the account belongs to a mint with permanent delegate.
/// This is a zero-size marker (no data) that indicates the token account's
/// mint has the SPL Token 2022 Permanent Delegate extension.
///
/// When present, token operations must check the SPL mint's PermanentDelegate
/// to determine the delegate authority before allowing transfers/burns.
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
pub struct PermanentDelegateAccountExtension;
