use light_compressed_account::Pubkey;
use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
#[repr(C)]
pub struct CompressibleExtensionInstructionData {
    /// In Epochs. (could do in slots as well)
    pub rent_payment: u64,
    pub has_rent_authority: u8,
    /// Authority that can close this account (in addition to owner)
    pub rent_authority: Pubkey,
    pub has_rent_recipient: u8,
    pub rent_recipient: Pubkey,
    pub has_top_up: u8,
    pub write_top_up: u32,
    pub payer_pda_bump: u8,
}
