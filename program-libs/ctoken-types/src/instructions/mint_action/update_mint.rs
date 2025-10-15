use light_compressed_account::Pubkey;
use light_zero_copy::ZeroCopy;

use crate::{AnchorDeserialize, AnchorSerialize};

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct UpdateAuthority {
    pub new_authority: Option<Pubkey>, // None = revoke authority, Some(key) = set new authority
}
