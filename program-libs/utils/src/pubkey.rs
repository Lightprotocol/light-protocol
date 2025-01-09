use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

#[derive(
    Debug,
    Copy,
    PartialEq,
    Clone,
    Immutable,
    FromBytes,
    IntoBytes,
    KnownLayout,
    BorshDeserialize,
    BorshSerialize,
    Default,
    Pod,
    Zeroable,
)]
#[repr(C)]
pub struct Pubkey(pub(crate) [u8; 32]);

#[cfg(not(feature = "anchor"))]
impl From<solana_program::pubkey::Pubkey> for Pubkey {
    fn from(pubkey: solana_program::pubkey::Pubkey) -> Self {
        Self(pubkey.to_bytes())
    }
}

#[cfg(feature = "anchor")]
impl From<anchor_lang::prelude::Pubkey> for Pubkey {
    fn from(pubkey: anchor_lang::prelude::Pubkey) -> Self {
        Self(pubkey.to_bytes())
    }
}
#[allow(clippy::from_over_into)]
#[cfg(feature = "anchor")]
impl Into<anchor_lang::prelude::Pubkey> for Pubkey {
    fn into(self) -> anchor_lang::prelude::Pubkey {
        anchor_lang::prelude::Pubkey::new_from_array(self.0)
    }
}

impl Pubkey {
    pub fn new_unique() -> Self {
        Self(solana_program::pubkey::Pubkey::new_unique().to_bytes())
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
}
