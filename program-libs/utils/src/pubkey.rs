use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};
use light_zero_copy::{borsh::Deserialize, errors::ZeroCopyError};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Ref};

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
    fn deserialize_at(bytes: &'a [u8]) -> Result<(Ref<&'a [u8], Pubkey>, &'a [u8]), ZeroCopyError> {
        Ok(Ref::<&[u8], Pubkey>::from_prefix(bytes)?)
    }
}

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
