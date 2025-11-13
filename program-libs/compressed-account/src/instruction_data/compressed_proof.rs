use light_zero_copy::{errors::ZeroCopyError, traits::ZeroCopyAt, ZeroCopyMut};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned};

#[repr(C)]
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
    Clone,
    Copy,
    PartialEq,
    Eq,
    KnownLayout,
    Immutable,
    FromBytes,
    IntoBytes,
    Unaligned,
    ZeroCopyMut,
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

impl CompressedProof {
    /// Convert the proof to a fixed-size byte array [u8; 128]
    pub fn to_array(&self) -> [u8; 128] {
        let mut result = [0u8; 128];
        result[0..32].copy_from_slice(&self.a);
        result[32..96].copy_from_slice(&self.b);
        result[96..128].copy_from_slice(&self.c);
        result
    }
}

impl TryFrom<&[u8]> for CompressedProof {
    type Error = crate::CompressedAccountError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() < 128 {
            return Err(crate::CompressedAccountError::InvalidProofSize(bytes.len()));
        }
        let mut a = [0u8; 32];
        let mut b = [0u8; 64];
        let mut c = [0u8; 32];
        a.copy_from_slice(&bytes[0..32]);
        b.copy_from_slice(&bytes[32..96]);
        c.copy_from_slice(&bytes[96..128]);
        Ok(Self { a, b, c })
    }
}

impl<'a> ZeroCopyAt<'a> for CompressedProof {
    type ZeroCopyAt = Ref<&'a [u8], Self>;
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
        Ok(Ref::<&[u8], CompressedProof>::from_prefix(bytes)?)
    }
}

#[cfg_attr(
    all(feature = "std", feature = "anchor"),
    derive(anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)
)]
#[cfg_attr(
    not(feature = "anchor"),
    derive(borsh::BorshDeserialize, borsh::BorshSerialize)
)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ValidityProof(pub Option<CompressedProof>);

impl ValidityProof {
    pub fn new(proof: Option<CompressedProof>) -> Self {
        Self(proof)
    }

    /// Convert the validity proof to a fixed-size byte array [u8; 128]
    /// Returns None if the proof is None
    pub fn to_array(&self) -> Option<[u8; 128]> {
        self.0.as_ref().map(|proof| proof.to_array())
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

impl TryFrom<&[u8]> for ValidityProof {
    type Error = crate::CompressedAccountError;

    /// Convert bytes to ValidityProof.
    /// Empty slice returns None, otherwise attempts to parse as CompressedProof and returns Some.
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.is_empty() {
            Ok(Self(None))
        } else {
            let proof = CompressedProof::try_from(bytes)?;
            Ok(Self(Some(proof)))
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<Option<CompressedProof>> for ValidityProof {
    fn into(self) -> Option<CompressedProof> {
        self.0
    }
}
