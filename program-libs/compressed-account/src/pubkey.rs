use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "bytemuck-des")]
use bytemuck::{Pod, Zeroable};
use light_zero_copy::{borsh::Deserialize, errors::ZeroCopyError};
use solana_program::pubkey;
use zerocopy::{little_endian::U64, FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned};
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
    BorshDeserialize,
    BorshSerialize,
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
    Clone,
    Immutable,
    FromBytes,
    IntoBytes,
    KnownLayout,
    BorshDeserialize,
    BorshSerialize,
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

impl AsRef<Pubkey> for Pubkey {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<'a> Deserialize<'a> for Pubkey {
    type Output = Ref<&'a [u8], Pubkey>;

    #[inline]
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Ref<&'a [u8], Pubkey>, &'a [u8]), ZeroCopyError> {
        Ok(Ref::<&[u8], Pubkey>::from_prefix(bytes)?)
    }
}

#[cfg(not(feature = "anchor"))]
impl From<pubkey::Pubkey> for Pubkey {
    fn from(pubkey: pubkey::Pubkey) -> Self {
        Self(pubkey.to_bytes())
    }
}
#[cfg(not(feature = "anchor"))]
impl From<&pubkey::Pubkey> for Pubkey {
    fn from(pubkey: &pubkey::Pubkey) -> Self {
        Self(pubkey.to_bytes())
    }
}

#[cfg(not(feature = "anchor"))]
impl From<Pubkey> for pubkey::Pubkey {
    fn from(pubkey: Pubkey) -> Self {
        pubkey::Pubkey::new_from_array(pubkey.to_bytes())
    }
}

#[cfg(not(feature = "anchor"))]
impl From<&Pubkey> for pubkey::Pubkey {
    fn from(pubkey: &Pubkey) -> Self {
        pubkey::Pubkey::new_from_array(pubkey.to_bytes())
    }
}
#[cfg(feature = "anchor")]
impl From<anchor_lang::prelude::Pubkey> for Pubkey {
    fn from(pubkey: pubkey::Pubkey) -> Self {
        Self(pubkey.to_bytes())
    }
}
#[cfg(feature = "anchor")]
impl From<crate::pubkey::Pubkey> for anchor_lang::prelude::Pubkey {
    fn from(pubkey: crate::pubkey::Pubkey) -> Self {
        anchor_lang::prelude::Pubkey::new_from_array(pubkey.to_bytes())
    }
}
#[cfg(feature = "anchor")]
impl From<&crate::pubkey::Pubkey> for anchor_lang::prelude::Pubkey {
    fn from(pubkey: &crate::pubkey::Pubkey) -> Self {
        anchor_lang::prelude::Pubkey::new_from_array(pubkey.to_bytes())
    }
}
#[cfg(feature = "anchor")]
impl From<&anchor_lang::prelude::Pubkey> for Pubkey {
    fn from(pubkey: &pubkey::Pubkey) -> Self {
        Self(pubkey.to_bytes())
    }
}

impl Pubkey {
    pub fn new_unique() -> Self {
        Self(pubkey::Pubkey::new_unique().to_bytes())
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
}

pub trait PubkeyTrait {
    fn trait_to_bytes(&self) -> [u8; 32];
    #[cfg(feature = "anchor")]
    fn to_anchor_pubkey(&self) -> anchor_lang::prelude::Pubkey;
}

impl PubkeyTrait for Pubkey {
    fn trait_to_bytes(&self) -> [u8; 32] {
        self.to_bytes()
    }
    #[cfg(feature = "anchor")]
    fn to_anchor_pubkey(&self) -> anchor_lang::prelude::Pubkey {
        self.into()
    }
}

#[cfg(feature = "anchor")]
impl PubkeyTrait for anchor_lang::prelude::Pubkey {
    fn trait_to_bytes(&self) -> [u8; 32] {
        self.to_bytes()
    }

    #[cfg(feature = "anchor")]
    fn to_anchor_pubkey(&self) -> Self {
        *self
    }
}

#[cfg(not(feature = "anchor"))]
impl PubkeyTrait for solana_program::pubkey::Pubkey {
    fn trait_to_bytes(&self) -> [u8; 32] {
        self.to_bytes()
    }
}

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};
pub trait ZeroCopyNumTrait:
    Add
    + Sub
    + AddAssign
    + SubAssign
    + Div
    + DivAssign
    + Mul
    + MulAssign
    + std::marker::Sized
    + From<u64>
    + Into<u64>
    + Copy
    + std::convert::TryFrom<u64>
{
    fn to_bytes_le(&self) -> [u8; 8];
}

impl ZeroCopyNumTrait for u64 {
    fn to_bytes_le(&self) -> [u8; 8] {
        self.to_le_bytes()
    }
}

impl ZeroCopyNumTrait for U64 {
    fn to_bytes_le(&self) -> [u8; 8] {
        self.to_bytes()
    }
}
