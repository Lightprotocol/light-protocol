use light_zero_copy::{borsh::Deserialize, errors::ZeroCopyError, ZeroCopyMut};
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

// Borsh compatible validity proof implementation. Use this in your anchor
// program unless you have zero-copy instruction data. Convert to zero-copy via
// `let proof = compression_params.proof.into();`.
//
// TODO: make the zerocopy implementation compatible with borsh serde via
// Anchor.
pub mod borsh_compat {
    use crate::{AnchorDeserialize, AnchorSerialize};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, AnchorDeserialize, AnchorSerialize)]
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

    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, AnchorDeserialize, AnchorSerialize)]
    pub struct ValidityProof(pub Option<CompressedProof>);

    impl ValidityProof {
        pub fn new(proof: Option<CompressedProof>) -> Self {
            Self(proof)
        }
    }

    impl From<super::CompressedProof> for CompressedProof {
        fn from(proof: super::CompressedProof) -> Self {
            Self {
                a: proof.a,
                b: proof.b,
                c: proof.c,
            }
        }
    }

    impl From<CompressedProof> for super::CompressedProof {
        fn from(proof: CompressedProof) -> Self {
            Self {
                a: proof.a,
                b: proof.b,
                c: proof.c,
            }
        }
    }

    impl From<super::ValidityProof> for ValidityProof {
        fn from(proof: super::ValidityProof) -> Self {
            Self(proof.0.map(|p| p.into()))
        }
    }

    impl From<ValidityProof> for super::ValidityProof {
        fn from(proof: ValidityProof) -> Self {
            Self(proof.0.map(|p| p.into()))
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
}
