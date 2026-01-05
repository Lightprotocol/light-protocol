use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize};

/// Marker extension indicating the account belongs to a pausable mint.
/// This is a zero-size marker (no data) that indicates the token account's
/// mint has the SPL Token 2022 Pausable extension.
///
/// When present, token operations must check the SPL mint's PausableConfig
/// to determine if the mint is paused before allowing transfers.
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
pub struct PausableAccountExtension;
