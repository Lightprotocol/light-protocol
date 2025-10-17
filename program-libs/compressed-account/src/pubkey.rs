#[cfg(feature = "bytemuck-des")]
use bytemuck::{Pod, Zeroable};
use light_zero_copy::{
    errors::ZeroCopyError,
    traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyStructInner, ZeroCopyStructInnerMut},
    ZeroCopyNew,
};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned};

#[cfg(feature = "bytemuck-des")]
#[cfg_attr(
    all(feature = "std", feature = "anchor"),
    derive(anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)
)]
#[cfg_attr(
    not(feature = "anchor"),
    derive(borsh::BorshDeserialize, borsh::BorshSerialize)
)]
#[derive(
    Pod,
    Zeroable,
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
    Default,
    Unaligned,
)]
#[repr(C)]
pub struct Pubkey(pub(crate) [u8; 32]);

#[cfg(not(feature = "bytemuck-des"))]
#[cfg_attr(
    all(feature = "std", feature = "anchor"),
    derive(anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)
)]
#[cfg_attr(
    not(feature = "anchor"),
    derive(borsh::BorshDeserialize, borsh::BorshSerialize)
)]
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
    Default,
    Unaligned,
)]
#[repr(C)]
pub struct Pubkey(pub(crate) [u8; 32]);

impl<'a> ZeroCopyNew<'a> for Pubkey {
    type ZeroCopyConfig = ();
    type Output = <Self as ZeroCopyAtMut<'a>>::ZeroCopyAtMut;
    fn byte_len(_config: &Self::ZeroCopyConfig) -> Result<usize, ZeroCopyError> {
        Ok(32)
    }
    fn new_zero_copy(
        bytes: &'a mut [u8],
        _config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        <Self as ZeroCopyAtMut<'a>>::zero_copy_at_mut(bytes)
    }
}

impl Pubkey {
    pub fn new_from_array(array: [u8; 32]) -> Self {
        Self(array)
    }

    pub fn new_from_slice(slice: &[u8]) -> Self {
        let mut array = [0u8; 32];
        array.copy_from_slice(slice);
        Self(array)
    }

    pub fn array_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

impl AsRef<Pubkey> for Pubkey {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl AsRef<[u8]> for Pubkey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl PartialEq<[u8; 32]> for Pubkey {
    fn eq(&self, other: &[u8; 32]) -> bool {
        self.0 == *other
    }
}

#[cfg(feature = "anchor")]
impl PartialEq<anchor_lang::prelude::Pubkey> for Pubkey {
    fn eq(&self, other: &anchor_lang::prelude::Pubkey) -> bool {
        self.0 == other.to_bytes()
    }
}

impl<'a> ZeroCopyAt<'a> for Pubkey {
    type ZeroCopyAt = Ref<&'a [u8], Pubkey>;

    #[inline]
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Ref<&'a [u8], Pubkey>, &'a [u8]), ZeroCopyError> {
        Ok(Ref::<&[u8], Pubkey>::from_prefix(bytes)?)
    }
}

impl<'a> ZeroCopyAtMut<'a> for Pubkey {
    type ZeroCopyAtMut = Ref<&'a mut [u8], Pubkey>;

    #[inline]
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
        Ok(Ref::<&mut [u8], Pubkey>::from_prefix(bytes)?)
    }
}

impl ZeroCopyStructInner for Pubkey {
    type ZeroCopyInner = Pubkey;
}

impl ZeroCopyStructInnerMut for Pubkey {
    type ZeroCopyInnerMut = Pubkey;
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

#[cfg(feature = "anchor")]
impl From<&anchor_lang::prelude::Pubkey> for Pubkey {
    fn from(pubkey: &anchor_lang::prelude::Pubkey) -> Self {
        Self::new_from_array(pubkey.to_bytes())
    }
}

#[cfg(feature = "anchor")]
impl From<anchor_lang::prelude::Pubkey> for Pubkey {
    fn from(pubkey: anchor_lang::prelude::Pubkey) -> Self {
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

impl Pubkey {
    #[cfg(feature = "new-unique")]
    pub fn new_unique() -> Self {
        Self(solana_pubkey::Pubkey::new_unique().to_bytes())
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
}

pub trait AsPubkey {
    fn to_pubkey_bytes(&self) -> [u8; 32];
    #[cfg(feature = "anchor")]
    fn to_anchor_pubkey(&self) -> anchor_lang::prelude::Pubkey;
}

impl AsPubkey for Pubkey {
    fn to_pubkey_bytes(&self) -> [u8; 32] {
        self.to_bytes()
    }
    #[cfg(feature = "anchor")]
    fn to_anchor_pubkey(&self) -> anchor_lang::prelude::Pubkey {
        self.into()
    }
}

#[cfg(feature = "anchor")]
impl AsPubkey for anchor_lang::prelude::Pubkey {
    fn to_pubkey_bytes(&self) -> [u8; 32] {
        self.to_bytes()
    }

    #[cfg(feature = "anchor")]
    fn to_anchor_pubkey(&self) -> Self {
        *self
    }
}

#[cfg(all(feature = "solana", not(feature = "anchor")))]
impl AsPubkey for solana_pubkey::Pubkey {
    fn to_pubkey_bytes(&self) -> [u8; 32] {
        self.to_bytes()
    }
}

impl AsPubkey for [u8; 32] {
    fn to_pubkey_bytes(&self) -> [u8; 32] {
        *self
    }
    #[cfg(feature = "anchor")]
    fn to_anchor_pubkey(&self) -> anchor_lang::prelude::Pubkey {
        (*self).into()
    }
}
