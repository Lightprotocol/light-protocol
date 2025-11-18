// TODO: try removing in separate PR
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
    /// Borsh-compatible ValidityProof. Use this in your anchor program unless
    /// you have zero-copy instruction data.
    pub struct ValidityProof(pub Option<CompressedProof>);

    impl ValidityProof {
        pub fn new(proof: Option<CompressedProof>) -> Self {
            Self(proof)
        }
    }

    impl From<light_compressed_account::instruction_data::compressed_proof::CompressedProof>
        for CompressedProof
    {
        fn from(
            proof: light_compressed_account::instruction_data::compressed_proof::CompressedProof,
        ) -> Self {
            Self {
                a: proof.a,
                b: proof.b,
                c: proof.c,
            }
        }
    }

    impl From<CompressedProof>
        for light_compressed_account::instruction_data::compressed_proof::CompressedProof
    {
        fn from(proof: CompressedProof) -> Self {
            Self {
                a: proof.a,
                b: proof.b,
                c: proof.c,
            }
        }
    }

    impl From<light_compressed_account::instruction_data::compressed_proof::ValidityProof>
        for ValidityProof
    {
        fn from(
            proof: light_compressed_account::instruction_data::compressed_proof::ValidityProof,
        ) -> Self {
            Self(proof.0.map(|p| p.into()))
        }
    }

    impl From<ValidityProof>
        for light_compressed_account::instruction_data::compressed_proof::ValidityProof
    {
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
