// use crate::pubkey;
use crate::{AnchorDeserialize, AnchorSerialize};
#[cfg(feature = "bytemuck-des")]
use bytemuck::{Pod, Zeroable};
use light_zero_copy::{borsh::Deserialize, errors::ZeroCopyError};
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

impl<'a> Deserialize<'a> for Pubkey {
    type Output = Ref<&'a [u8], Pubkey>;

    #[inline]
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Ref<&'a [u8], Pubkey>, &'a [u8]), ZeroCopyError> {
        Ok(Ref::<&[u8], Pubkey>::from_prefix(bytes)?)
    }
}

#[cfg(any(feature = "anchor", feature = "solana", feature = "pinocchio"))]
impl From<crate::Pubkey> for Pubkey {
    fn from(pubkey: crate::pubkey::Pubkey) -> Self {
        Self(pubkey.to_bytes())
    }
}
#[cfg(any(feature = "anchor", feature = "solana", feature = "pinocchio"))]
impl From<&crate::Pubkey> for Pubkey {
    fn from(pubkey: &crate::pubkey::Pubkey) -> Self {
        Self(pubkey.to_bytes())
    }
}

#[cfg(any(feature = "anchor", feature = "solana", feature = "pinocchio"))]
impl From<Pubkey> for crate::Pubkey {
    fn from(pubkey: Pubkey) -> Self {
        crate::Pubkey::new_from_array(pubkey.to_bytes())
    }
}

#[cfg(any(feature = "anchor", feature = "solana", feature = "pinocchio"))]
impl From<&Pubkey> for crate::Pubkey {
    fn from(pubkey: &Pubkey) -> Self {
        crate::Pubkey::new_from_array(pubkey.to_bytes())
    }
}
// #[cfg(feature = "anchor")]
// impl From<anchor_lang::prelude::Pubkey> for Pubkey {
//     fn from(pubkey: pubkey::Pubkey) -> Self {
//         Self(pubkey.to_bytes())
//     }
// }
// #[cfg(feature = "anchor")]
// impl From<crate::pubkey::Pubkey> for anchor_lang::prelude::Pubkey {
//     fn from(pubkey: crate::pubkey::Pubkey) -> Self {
//         anchor_lang::prelude::Pubkey::new_from_array(pubkey.to_bytes())
//     }
// }
// #[cfg(feature = "anchor")]
// impl From<&crate::pubkey::Pubkey> for anchor_lang::prelude::Pubkey {
//     fn from(pubkey: &crate::pubkey::Pubkey) -> Self {
//         anchor_lang::prelude::Pubkey::new_from_array(pubkey.to_bytes())
//     }
// }
// #[cfg(feature = "anchor")]
// impl From<&anchor_lang::prelude::Pubkey> for Pubkey {
//     fn from(pubkey: &pubkey::Pubkey) -> Self {
//         Self(pubkey.to_bytes())
//     }
// }
impl Pubkey {
    #[cfg(any(feature = "anchor", feature = "solana", feature = "pinocchio"))]
    pub fn new_unique() -> Self {
        Self(crate::Pubkey::new_unique().to_bytes())
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
}
