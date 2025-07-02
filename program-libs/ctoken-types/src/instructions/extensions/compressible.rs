use light_compressed_account::Pubkey;
use light_zero_copy::{ZeroCopy, ZeroCopyMut};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::{AnchorDeserialize, AnchorSerialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    AnchorSerialize,
    AnchorDeserialize,
    ZeroCopy,
    ZeroCopyMut,
    KnownLayout,
    Immutable,
    FromBytes,
    IntoBytes,
)]
#[repr(C)]
pub struct CompressibleExtensionInstructionData {
    /// Number of slots that must pass before compression is allowed
    pub slots_until_compression: u64,
    /// Authority that can close this account (in addition to owner)
    pub rent_authority: Pubkey,
    pub rent_recipient: Pubkey,
}
