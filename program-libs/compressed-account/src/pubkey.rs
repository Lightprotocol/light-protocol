use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "bytemuck-des")]
use bytemuck::{Pod, Zeroable};
use light_hasher::{to_byte_array::ToByteArray, HasherError};
use light_zero_copy::{borsh::Deserialize, borsh_mut::DeserializeMut, errors::ZeroCopyError};
use solana_program::pubkey;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned};
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

impl Deserialize for Pubkey {
    type Output<'a> = Ref<&'a [u8], Pubkey>;

    #[inline]
    fn zero_copy_at<'a>(
        bytes: &'a [u8],
    ) -> Result<(Ref<&'a [u8], Pubkey>, &'a [u8]), ZeroCopyError> {
        Ok(Ref::<&[u8], Pubkey>::from_prefix(bytes)?)
    }
}

impl DeserializeMut for Pubkey {
    type Output<'a> = Ref<&'a mut [u8], Pubkey>;

    #[inline]
    fn zero_copy_at_mut<'a>(
        bytes: &'a mut [u8],
    ) -> Result<(Self::Output<'a>, &'a mut [u8]), ZeroCopyError> {
        Ok(Ref::<&'a mut [u8], Pubkey>::from_prefix(bytes)?)
    }
}
impl PartialEq<<Pubkey as Deserialize>::Output<'_>> for Pubkey {
    fn eq(&self, other: &<Pubkey as Deserialize>::Output<'_>) -> bool {
        self.0 == other.0
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

impl ToByteArray for Pubkey {
    const NUM_FIELDS: usize = 1;

    fn to_byte_array(&self) -> Result<[u8; 32], HasherError> {
        Ok(self.to_bytes())
    }

    fn to_byte_arrays<const NUM_FIELDS: usize>(
        &self,
    ) -> Result<[[u8; 32]; NUM_FIELDS], HasherError> {
        if Self::NUM_FIELDS != NUM_FIELDS {
            return Err(HasherError::InvalidNumFields);
        }
        Ok([self.to_byte_array()?; NUM_FIELDS])
    }
}
