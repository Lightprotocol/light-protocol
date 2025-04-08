#[cfg(feature = "bytemuck-des")]
use bytemuck::{Pod, Zeroable};
use light_zero_copy::{borsh::Deserialize, errors::ZeroCopyError};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned};

use crate::{AnchorDeserialize, AnchorSerialize};
#[cfg(feature = "bytemuck-des")]
#[derive(
    Pod,
    Zeroable,
    Debug,
    Copy,
    PartialEq,
    Clone,
    Immutable,
    FromBytes,
    IntoBytes,
    KnownLayout,
    AnchorDeserialize,
    AnchorSerialize,
    Default,
    Unaligned,
)]
#[repr(C)]
pub struct Pubkey(pub(crate) [u8; 32]);

#[cfg(not(feature = "bytemuck-des"))]
#[derive(
    Debug,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Clone,
    Immutable,
    FromBytes,
    IntoBytes,
    KnownLayout,
    AnchorDeserialize,
    AnchorSerialize,
    Default,
    Unaligned,
)]
#[repr(C)]
pub struct Pubkey(pub(crate) [u8; 32]);

impl Pubkey {
    pub fn new_from_array(array: [u8; 32]) -> Self {
        Self(array)
    }

    pub fn new_from_slice(slice: &[u8]) -> Self {
        let mut array = [0u8; 32];
        array.copy_from_slice(slice);
        Self(array)
    }
}

impl<'a> Deserialize<'a> for Pubkey {
    type Output = Ref<&'a [u8], Pubkey>;

    #[inline]
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Ref<&'a [u8], Pubkey>, &'a [u8]), ZeroCopyError> {
        Ok(Ref::<&[u8], Pubkey>::from_prefix(bytes)?)
    }
}
impl From<Pubkey> for [u8; 32] {
    fn from(pubkey: Pubkey) -> Self {
        pubkey.to_bytes()
    }
}

impl From<&Pubkey> for [u8; 32] {
    fn from(pubkey: &Pubkey) -> Self {
        pubkey.to_bytes()
    }
}

impl From<[u8; 32]> for Pubkey {
    fn from(pubkey: [u8; 32]) -> Self {
        Self(pubkey)
    }
}

impl From<&[u8; 32]> for Pubkey {
    fn from(pubkey: &[u8; 32]) -> Self {
        Self(*pubkey)
    }
}

#[cfg(not(feature = "anchor"))]
impl From<Pubkey> for solana_program::pubkey::Pubkey {
    fn from(pubkey: Pubkey) -> Self {
        Self::new_from_array(pubkey.to_bytes())
    }
}

#[cfg(not(feature = "anchor"))]
impl From<&Pubkey> for solana_program::pubkey::Pubkey {
    fn from(pubkey: &Pubkey) -> Self {
        Self::new_from_array(pubkey.to_bytes())
    }
}

#[cfg(feature = "anchor")]
impl From<Pubkey> for anchor_lang::prelude::Pubkey {
    fn from(pubkey: Pubkey) -> Self {
        Self::new_from_array(pubkey.to_bytes())
    }
}

#[cfg(feature = "anchor")]
impl From<&Pubkey> for anchor_lang::prelude::Pubkey {
    fn from(pubkey: &Pubkey) -> Self {
        Self::new_from_array(pubkey.to_bytes())
    }
}

#[cfg(not(feature = "pinocchio"))]
impl From<crate::Pubkey> for Pubkey {
    fn from(pubkey: crate::Pubkey) -> Self {
        Self(pubkey.to_bytes())
    }
}

#[cfg(not(feature = "pinocchio"))]
impl From<&crate::Pubkey> for Pubkey {
    fn from(pubkey: &crate::Pubkey) -> Self {
        Self(pubkey.to_bytes())
    }
}

impl Pubkey {
    #[cfg(not(feature = "pinocchio"))]
    pub fn new_unique() -> Self {
        Self(solana_program::pubkey::Pubkey::new_unique().to_bytes())
    }

    #[cfg(feature = "pinocchio")]
    pub fn new_unique() -> Self {
        // Just generate a random pubkey
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 32];
        rng.fill(&mut bytes);
        Self(bytes)
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
}
