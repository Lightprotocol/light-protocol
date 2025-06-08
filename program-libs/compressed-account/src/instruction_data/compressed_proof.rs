use light_zero_copy::{borsh::Deserialize, errors::ZeroCopyError};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned};

use crate::{AnchorDeserialize, AnchorSerialize};

#[repr(C)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    AnchorDeserialize,
    AnchorSerialize,
    KnownLayout,
    Immutable,
    FromBytes,
    IntoBytes,
    Unaligned,
)]
pub struct CompressedProof {
    pub a: [u8; 32],
    pub b: [u8; 64],
    pub c: [u8; 32],
}

impl Default for CompressedProof {
    fn default() -> Self {
        Self {
            a: [0; 32],
            b: [0; 64],
            c: [0; 32],
        }
    }
}

impl<'a> Deserialize<'a> for CompressedProof {
    type Output = Ref<&'a [u8], Self>;
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), ZeroCopyError> {
        Ok(Ref::<&[u8], CompressedProof>::from_prefix(bytes)?)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, AnchorDeserialize, AnchorSerialize)]
pub struct ValidityProof(pub Option<CompressedProof>);

impl ValidityProof {
    pub fn new(proof: Option<CompressedProof>) -> Self {
        Self(proof)
    }
}

impl From<CompressedProof> for ValidityProof {
    fn from(proof: CompressedProof) -> Self {
        Self(Some(proof))
    }
}

impl From<Option<CompressedProof>> for ValidityProof {
    fn from(proof: Option<CompressedProof>) -> Self {
        Self(proof)
    }
}
impl From<&CompressedProof> for ValidityProof {
    fn from(proof: &CompressedProof) -> Self {
        Self(Some(*proof))
    }
}

impl From<&Option<CompressedProof>> for ValidityProof {
    fn from(proof: &Option<CompressedProof>) -> Self {
        Self(*proof)
    }
}

#[allow(clippy::from_over_into)]
impl Into<Option<CompressedProof>> for ValidityProof {
    fn into(self) -> Option<CompressedProof> {
        self.0
    }
}
